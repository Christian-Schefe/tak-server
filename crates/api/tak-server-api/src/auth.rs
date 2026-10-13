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
    workflow::account::authentication::{LoginError, RecoverError, RegisterError, ResetError},
};

use crate::{AppState, ServiceError};

pub struct Auth {
    pub user_id: UserId,
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
            match app.auth.validate_account_jwt(bearer.token()) {
                Some(user_id) => {
                    return Ok(Auth { user_id });
                }
                None => {
                    return Err(ServiceError::Unauthorized(
                        "Authentication failed".to_string(),
                    ));
                }
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
        .route("/recover", post(recover))
        .route("/reset", post(reset))
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
    let user_id = match app
        .app
        .authentication_use_case
        .login(req.identifier, req.password)
        .await
    {
        Ok(user_id) => user_id,
        Err(LoginError::InvalidCredentials) => {
            return Err(ServiceError::Unauthorized(
                "Invalid identifier or password".to_string(),
            ));
        }
        Err(LoginError::Internal) => {
            return Err(ServiceError::Internal(format!("Failed to login",)));
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
    let user_id = match app
        .app
        .authentication_use_case
        .register(req.email, req.username, req.password)
        .await
    {
        Ok(user_id) => user_id,
        Err(RegisterError::Internal) => {
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

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecoverRequest {
    identifier: String,
}

async fn recover(
    State(app): State<AppState>,
    Json(req): Json<RecoverRequest>,
) -> Result<(), ServiceError> {
    match app
        .app
        .authentication_use_case
        .recover(req.identifier)
        .await
    {
        Ok(()) => Ok(()),
        Err(RecoverError::AccountNotFound) => {
            Err(ServiceError::NotFound("Account not found".to_string()))
        }
        Err(RecoverError::EmailNotSet) => Err(ServiceError::NotPossible(
            "Email not set for account".to_string(),
        )),
        Err(RecoverError::Internal) => Err(ServiceError::Internal(
            "Failed to initiate account recovery".to_string(),
        )),
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResetRequest {
    identifier: String,
    token: String,
    new_password: String,
}

async fn reset(
    State(app): State<AppState>,
    Json(req): Json<ResetRequest>,
) -> Result<(), ServiceError> {
    match app
        .app
        .authentication_use_case
        .reset(req.identifier, req.token, req.new_password)
        .await
    {
        Ok(()) => Ok(()),
        Err(ResetError::InvalidToken) => Err(ServiceError::Unauthorized(
            "Invalid token or password".to_string(),
        )),
        Err(ResetError::Internal) => Err(ServiceError::Internal(
            "Failed to reset password".to_string(),
        )),
        Err(ResetError::AccountNotFound) => Err(ServiceError::NotFound(
            "Invalid token or password".to_string(),
        )),
    }
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
    let (user_id, new_guest) = match auth {
        Ok(auth) => (auth.user_id, false),
        Err(e) => {
            if let Some(true) = params.prevent_guest {
                return Err(e);
            }
            match app.app.authentication_use_case.create_guest_account().await {
                Ok(guest_account) => (guest_account, true),
                Err(RegisterError::Internal) => {
                    return Err(ServiceError::Internal(
                        "Failed to create guest account".to_string(),
                    ));
                }
            }
        }
    };
    let moderation = app
        .app
        .account_moderate_use_case
        .get_moderation(user_id)
        .await
        .map_err(|_| ServiceError::Internal("Failed to retrieve moderation info".to_string()))?;

    Ok(Json(IdentityInfo {
        user_id: user_id.to_string(),
        is_guest: moderation.is_guest(),
        is_admin: moderation.is_admin(),
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
pub trait ApiAuthPort {
    fn validate_account_jwt(&self, token: &str) -> Option<UserId>;
    fn generate_account_jwt(&self, id: UserId, duration: Duration) -> String;
}
