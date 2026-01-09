use axum::{
    extract::FromRequestParts,
    http::{header::COOKIE, request::Parts},
};
use tak_server_app::ports::authentication::Account;

use crate::{AppState, ServiceError};

pub struct Auth(pub Account);

impl FromRequestParts<AppState> for Auth {
    type Rejection = ServiceError;

    async fn from_request_parts(
        parts: &mut Parts,
        app: &AppState,
    ) -> Result<Self, Self::Rejection> {
        log::debug!("Headers: {:?}", parts.headers);

        if let Some(cookie) = parts.headers.get(COOKIE)
            && let Ok(cookie) = cookie.to_str()
        {
            if let Ok(acc) = verify_kratos_cookie(app, cookie).await {
                return Ok(Auth(acc));
            }
        }

        if let Some(auth_header) = parts.headers.get("authorization") {
            if let Ok(token) = auth_header.to_str() {
                if let Ok(acc) = verify_guest_jwt(app, token).await {
                    return Ok(Auth(acc));
                }
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

async fn verify_guest_jwt(app: &AppState, _token: &str) -> Result<Account, ()> {
    let account = app.auth.get_account_by_guest_jwt(_token).await.ok_or(())?;
    Ok(account)
}

#[async_trait::async_trait]
pub trait ApiAuthPort {
    async fn get_account_by_kratos_cookie(&self, token: &str) -> Option<Account>;
    async fn get_account_by_guest_jwt(&self, token: &str) -> Option<Account>;
}
