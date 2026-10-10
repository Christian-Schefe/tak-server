use std::time::Duration;

use axum::{
    Json, RequestPartsExt,
    extract::{FromRequestParts, Query, State},
    http::request::Parts,
    routing::{get, post},
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use tak_server_api_contract::auth::IdentityInfo;
use tak_server_app::{
    domain::UserId,
    ports::authentication::{Account, AuthenticationPort},
};

use crate::{AppState, ServiceError};

pub struct Auth {
    pub user_id: UserId,
    pub account: Account,
}

impl FromRequestParts<AppState> for Auth {
    type Rejection = ServiceError;

    async fn from_request_parts(
        parts: &mut Parts,
        app: &AppState,
    ) -> Result<Self, Self::Rejection> {
        if let Ok(TypedHeader(Authorization(bearer))) =
            parts.extract::<TypedHeader<Authorization<Bearer>>>().await
        {
            if let Some((user_id, acc)) = app.auth.validate_account_jwt(bearer.token()).await {
                return Ok(Auth {
                    user_id,
                    account: acc,
                });
            }
        }

        Err(ServiceError::Unauthorized(
            "Authentication failed".to_string(),
        ))
    }
}

pub fn register_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/login", post(login))
        .route("/register", post(register))
        .route("/api-token", post(create_api_token))
        .route("/whoami", get(who_am_i))
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginRequest {
    identifier: String,
    password: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginResponse {
    jwt: String,
}

async fn login(
    State(app): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ServiceError> {
    let (user_id, _) = match app.auth.login_account(&req.identifier, &req.password).await {
        Some(account) => account,
        None => {
            return Err(ServiceError::Unauthorized(
                "Invalid identifier or password".to_string(),
            ));
        }
    };

    Ok(Json(LoginResponse {
        jwt: app
            .auth
            .generate_account_jwt(user_id, std::time::Duration::from_secs(60 * 60 * 24)),
    }))
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegisterRequest {
    email: String,
    username: String,
    password: String,
}

async fn register(
    State(app): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<LoginResponse>, ServiceError> {
    let (user_id, _) = match app
        .auth
        .register_account(&req.email, &req.username, &req.password)
        .await
    {
        Some(account) => account,
        None => {
            return Err(ServiceError::Internal(
                "Failed to register account".to_string(),
            ));
        }
    };

    Ok(Json(LoginResponse {
        jwt: app
            .auth
            .generate_account_jwt(user_id, std::time::Duration::from_secs(60 * 60 * 24)),
    }))
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct WhoAmIQueryParams {
    prevent_guest: Option<bool>,
}

async fn who_am_i(
    auth: Result<Auth, ServiceError>,
    State(app): State<AppState>,
    Query(params): Query<WhoAmIQueryParams>,
) -> Result<Json<IdentityInfo>, ServiceError> {
    let ((user_id, account), new_guest) = match auth {
        Ok(auth) => ((auth.user_id, auth.account), false),
        Err(e) => {
            if let Some(true) = params.prevent_guest {
                return Err(e);
            }
            match app.auth.create_guest_account().await {
                Some(guest_account) => (guest_account, true),
                None => {
                    return Err(ServiceError::Internal(
                        "Failed to create guest account".to_string(),
                    ));
                }
            }
        }
    };
    Ok(Json(IdentityInfo {
        user_id: user_id.to_string(),
        is_guest: account.is_guest(),
        is_admin: account.is_admin(),
        new_guest,
        jwt: app
            .auth
            .generate_account_jwt(user_id, std::time::Duration::from_secs(60 * 60 * 24)),
    }))
}

async fn create_api_token(
    State(app): State<AppState>,
    auth: Auth,
    Json(req): Json<CreateApiTokenRequest>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    if req.duration == 0 || req.duration > 60 * 60 * 24 * 365 {
        return Err(ServiceError::BadRequest(
            "Duration must be between 1 second and 365 days".to_string(),
        ));
    }
    let cert = app
        .auth
        .generate_account_jwt(auth.user_id, std::time::Duration::from_secs(req.duration));
    Ok(Json(serde_json::json!({ "certificate": cert })))
}

#[derive(serde::Deserialize)]
struct CreateApiTokenRequest {
    duration: u64,
}

#[async_trait::async_trait]
pub trait ApiAuthPort: AuthenticationPort {
    async fn register_account(
        &self,
        email: &str,
        username: &str,
        password: &str,
    ) -> Option<(UserId, Account)>;
    async fn login_account(&self, identifier: &str, password: &str) -> Option<(UserId, Account)>;
    async fn create_guest_account(&self) -> Option<(UserId, Account)>;

    fn generate_account_jwt(&self, id: UserId, duration: Duration) -> String;
    async fn validate_account_jwt(&self, token: &str) -> Option<(UserId, Account)>;
}
