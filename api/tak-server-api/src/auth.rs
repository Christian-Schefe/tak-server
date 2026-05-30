use std::time::Duration;

use axum::{
    Json, RequestPartsExt,
    extract::{FromRequestParts, State},
    http::{header::COOKIE, request::Parts},
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use tak_server_app::{
    domain::{AccountId, moderation::AccountRole},
    ports::authentication::{Account, AuthenticationPort},
};

use crate::{AppState, ServiceError};

pub struct StrictAuth {
    pub account: Option<Account>,
}

impl FromRequestParts<AppState> for StrictAuth {
    type Rejection = ServiceError;

    async fn from_request_parts(
        parts: &mut Parts,
        app: &AppState,
    ) -> Result<Self, Self::Rejection> {
        if let Some(cookie) = parts.headers.get(COOKIE)
            && let Ok(cookie) = cookie.to_str()
        {
            if let Ok(acc) = verify_kratos_cookie(app, cookie).await {
                return Ok(StrictAuth { account: Some(acc) });
            }
        }

        if let Ok(TypedHeader(Authorization(bearer))) =
            parts.extract::<TypedHeader<Authorization<Bearer>>>().await
        {
            if let Some(acc) = app.auth.validate_account_jwt(bearer.token()).await {
                if acc.is_guest() || acc.is_bot() {
                    return Ok(StrictAuth { account: Some(acc) });
                } else {
                    tracing::info!(?acc, "Rejected non-(guest-or-bot) JWT for strict auth");
                }
            } else {
                tracing::info!("Failed to validate JWT for strict auth");
            }
        }

        Ok(StrictAuth { account: None })
    }
}

pub struct Auth {
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
            if let Some(acc) = app.auth.validate_account_jwt(bearer.token()).await {
                return Ok(Auth { account: acc });
            }
        }

        Err(ServiceError::Unauthorized(
            "Authentication failed".to_string(),
        ))
    }
}

async fn verify_kratos_cookie(app: &AppState, cookie: &str) -> Result<Account, ()> {
    let account = app
        .auth
        .get_account_by_kratos_cookie(cookie)
        .await
        .ok_or(())?;
    Ok(account)
}

pub async fn get_bot_certificate(
    State(app): State<AppState>,
    auth: StrictAuth,
    Json(req): Json<GetBotCertificateRequest>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let account = auth
        .account
        .ok_or_else(|| ServiceError::Unauthorized("Authentication required".to_string()))?;
    if !matches!(account.role, AccountRole::Admin) {
        return Err(ServiceError::Unauthorized(
            "Admin role required".to_string(),
        ));
    }
    let Ok(target_account_id) = AccountId::try_from(req.target_account_id.clone()) else {
        return Err(ServiceError::BadRequest(
            "Invalid target account ID format".to_string(),
        ));
    };
    let target_account = app
        .auth
        .get_account(&target_account_id)
        .await
        .ok_or_else(|| ServiceError::NotFound("Target account not found".to_string()))?;
    if !target_account.is_bot() {
        return Err(ServiceError::BadRequest(
            "Target account is not a bot".to_string(),
        ));
    }
    let cert = app.auth.generate_account_jwt(
        &target_account.account_id,
        std::time::Duration::from_secs(60 * 60 * 24 * 365),
    );
    Ok(Json(serde_json::json!({ "certificate": cert })))
}

#[derive(serde::Deserialize)]
pub struct GetBotCertificateRequest {
    pub target_account_id: String,
}

#[async_trait::async_trait]
pub trait ApiAuthPort: AuthenticationPort {
    async fn get_account_by_kratos_cookie(&self, token: &str) -> Option<Account>;
    async fn create_guest(&self) -> Option<Account>;

    fn generate_account_jwt(&self, id: &AccountId, duration: Duration) -> String;
    async fn validate_account_jwt(&self, token: &str) -> Option<Account>;

    async fn get_account_by_username(&self, username: &str) -> Option<Account>;
}
