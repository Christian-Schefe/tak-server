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

pub struct Auth {
    pub account: Account,
    pub guest_jwt: Option<String>,
}

impl FromRequestParts<AppState> for Auth {
    type Rejection = ServiceError;

    async fn from_request_parts(
        parts: &mut Parts,
        app: &AppState,
    ) -> Result<Self, Self::Rejection> {
        if let Some(cookie) = parts.headers.get(COOKIE)
            && let Ok(cookie) = cookie.to_str()
        {
            if let Ok(acc) = verify_kratos_cookie(app, cookie).await {
                return Ok(Auth {
                    account: acc,
                    guest_jwt: None,
                });
            }
        }

        if let Ok(TypedHeader(Authorization(bearer))) =
            parts.extract::<TypedHeader<Authorization<Bearer>>>().await
        {
            if let Ok(acc) = verify_guest_jwt(app, bearer.token()).await {
                return Ok(Auth {
                    account: acc,
                    guest_jwt: Some(bearer.token().to_string()),
                });
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
    let account = app.auth.get_account_by_guest_jwt(_token).ok_or(())?;
    Ok(account)
}

#[async_trait::async_trait]
pub trait ApiAuthPort: AuthenticationPort {
    async fn get_account_by_kratos_cookie(&self, token: &str) -> Option<Account>;
    fn get_account_by_guest_jwt(&self, token: &str) -> Option<Account>;
    fn generate_or_refresh_guest_jwt(&self, token: Option<&str>) -> String;

    fn generate_account_jwt(&self, id: &AccountId) -> String;
    fn validate_account_jwt(&self, token: &str) -> Option<AccountId>;

    async fn get_account_by_username(&self, username: &str) -> Option<Account>;
}
