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
use url::Url;

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
            match app.auth.validate_account_jwt(bearer.token()).await {
                Ok(Some((user_id, acc))) => {
                    return Ok(Auth {
                        user_id,
                        account: acc,
                    });
                }
                Ok(None) => {
                    return Err(ServiceError::Unauthorized(
                        "Authentication failed".to_string(),
                    ));
                }
                Err(e) => {
                    tracing::error!("Error validating JWT: {:?}", e);
                    return Err(ServiceError::Internal("Failed to validate JWT".to_string()));
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
    let (user_id, _) = match app.auth.login_account(&req.identifier, &req.password).await {
        Ok(Some(account)) => account,
        Ok(None) => {
            return Err(ServiceError::Unauthorized(
                "Invalid identifier or password".to_string(),
            ));
        }
        Err(e) => {
            return Err(ServiceError::Internal(format!(
                "Failed to login account: {:?}",
                e
            )));
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
        Ok(account) => account,
        Err(_) => {
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
    let (user_id, account) = match app.auth.find_account(&req.identifier).await {
        Ok(Some(account)) => account,
        Ok(None) => {
            return Err(ServiceError::NotFound(
                "Account with this identifier not found".to_string(),
            ));
        }
        Err(e) => {
            tracing::error!("Error finding account: {:?}", e);
            return Err(ServiceError::Internal("Failed to find account".to_string()));
        }
    };

    let Some(email) = account.email else {
        return Err(ServiceError::BadRequest(
            "Account does not have an email address".to_string(),
        ));
    };

    let token = app
        .auth
        .generate_reset_token(user_id, Duration::from_mins(30))
        .map_err(|_| ServiceError::Internal("Failed to generate reset token".to_string()))?;

    let site_address =
        std::env::var("SITE_ADDRESS").expect("SITE_ADDRESS environment variable must be set");

    let mut base_url = Url::parse(&format!("https://{}/reset", site_address)).unwrap();
    base_url.query_pairs_mut().append_pair("token", &token);

    let body = format!(
        "Hello {},\n\nYou requested a password reset. Please click the link below to login and reset your password:\n\n{}\n\nThe link will expire in 30 minutes.\n\nIf you did not request this, please ignore this email.",
        account.username, base_url
    );

    app.email
        .send_email(&email, "Password Reset", &body)
        .map_err(|e| {
            ServiceError::Internal(format!("Failed to send password reset email: {:?}", e))
        })?;

    Ok(())
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
    let user_id = match app.auth.find_account(&req.identifier).await {
        Ok(Some((user_id, _))) => user_id,
        Ok(None) => {
            return Err(ServiceError::NotFound(
                "Account with this identifier not found".to_string(),
            ));
        }
        Err(e) => {
            tracing::error!("Error finding account: {:?}", e);
            return Err(ServiceError::Internal("Failed to find account".to_string()));
        }
    };
    app.auth
        .reset_account_password(user_id, &req.token, &req.new_password)
        .await
        .map_err(|_| ServiceError::BadRequest("Invalid token or password".to_string()))?;

    Ok(())
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
                Ok(guest_account) => (guest_account, true),
                Err(e) => {
                    tracing::error!("Error creating guest account: {:?}", e);
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
    ) -> Result<(UserId, Account), String>;
    async fn login_account(
        &self,
        identifier: &str,
        password: &str,
    ) -> Result<Option<(UserId, Account)>, String>;
    async fn create_guest_account(&self) -> Result<(UserId, Account), String>;
    async fn find_account(&self, identifier: &str) -> Result<Option<(UserId, Account)>, String>;
    async fn reset_account_password(
        &self,
        user_id: UserId,
        token: &str,
        new_password: &str,
    ) -> Result<Option<()>, String>;

    fn generate_reset_token(&self, user_id: UserId, duration: Duration) -> Result<String, String>;
    fn generate_account_jwt(&self, id: UserId, duration: Duration) -> String;
    async fn validate_account_jwt(&self, token: &str) -> Result<Option<(UserId, Account)>, String>;
}
