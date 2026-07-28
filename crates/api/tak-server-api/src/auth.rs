use std::time::Duration;

use axum::{
    Json, RequestPartsExt,
    extract::{FromRequestParts, Query, State},
    http::{header::COOKIE, request::Parts},
    routing::{get, post},
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use tak_server_api_contract::auth::IdentityInfo;
use tak_server_app::{
    domain::{AccountId, moderation::AccountRole},
    ports::authentication::{Account, AuthenticationPort},
    services::player_resolver::ResolveError,
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
            } else {
                tracing::info!("Failed to verify Kratos cookie for strict auth");
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

pub fn register_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/bot-certificate", post(get_bot_certificate))
        .route("/whoami", get(who_am_i))
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhoAmIQueryParams {
    pub prevent_guest: Option<bool>,
}

async fn who_am_i(
    auth: StrictAuth,
    State(app): State<AppState>,
    Query(params): Query<WhoAmIQueryParams>,
) -> Result<Json<IdentityInfo>, ServiceError> {
    let new_guest = auth.account.is_none();
    if let Some(true) = params.prevent_guest
        && auth.account.as_ref().is_none_or(|acc| acc.is_guest())
    {
        return Err(ServiceError::Unauthorized(
            "Guest accounts are not allowed".to_string(),
        ));
    }
    let account = if let Some(account) = auth.account {
        account
    } else {
        match app.auth.create_guest().await {
            Some(guest_account) => guest_account,
            None => {
                return Err(ServiceError::Internal(
                    "Failed to create guest account".to_string(),
                ));
            }
        }
    };
    let player_id = app
        .app
        .player_resolver_service
        .resolve_player_id_by_account_id(&account.account_id)
        .await
        .map_err(|ResolveError::Internal| {
            ServiceError::Internal("Failed to resolve player ID".to_string())
        })?;
    Ok(Json(IdentityInfo {
        account_id: account.account_id.to_string(),
        player_id: player_id.to_string(),
        is_guest: account.is_guest(),
        is_admin: account.is_admin(),
        new_guest,
        jwt: app.auth.generate_account_jwt(
            &account.account_id,
            std::time::Duration::from_secs(60 * 60 * 24),
        ),
    }))
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
