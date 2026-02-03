use axum::{
    RequestPartsExt,
    extract::FromRequestParts,
    http::{header::COOKIE, request::Parts},
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use tak_server_app::{
    domain::AccountId,
    ports::authentication::{Account, AuthenticationPort},
};

use crate::{AppState, ServiceError};

pub struct StrictAuth {
    pub account: Account,
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
                return Ok(StrictAuth { account: acc });
            }
        }

        if let Ok(TypedHeader(Authorization(bearer))) =
            parts.extract::<TypedHeader<Authorization<Bearer>>>().await
        {
            if let Some(acc_id) = app.auth.validate_account_jwt(bearer.token())
                && let Some(acc) = app.auth.get_account(&acc_id).await
            {
                if acc.is_guest() {
                    return Ok(StrictAuth { account: acc });
                } else {
                    log::info!("Rejected non-guest JWT for strict auth: {:?}", acc);
                }
            } else {
                log::info!("Failed to validate JWT for strict auth");
            }
        }

        Err(ServiceError::Unauthorized(
            "Authentication failed".to_string(),
        ))
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
            if let Some(acc_id) = app.auth.validate_account_jwt(bearer.token())
                && let Some(acc) = app.auth.get_account(&acc_id).await
            {
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

#[async_trait::async_trait]
pub trait ApiAuthPort: AuthenticationPort {
    async fn get_account_by_kratos_cookie(&self, token: &str) -> Option<Account>;
    fn create_guest(&self) -> Account;

    fn generate_account_jwt(&self, id: &AccountId) -> String;
    fn validate_account_jwt(&self, token: &str) -> Option<AccountId>;

    async fn get_account_by_username(&self, username: &str) -> Option<Account>;
}
