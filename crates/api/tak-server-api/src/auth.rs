use std::sync::LazyLock;
use std::time::Duration;

use axum::{
    Json, RequestPartsExt,
    extract::{FromRequestParts, State},
    http::request::Parts,
    routing::post,
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use tak_domain::UserId;
use tak_server_app::workflow::account::authentication::{
    LoginError, LogoutError, RecoverError, RefreshError, RegisterError, ResetError,
};

use crate::{AppState, ServiceError};

pub struct Auth {
    pub user_id: UserId,
}

impl FromRequestParts<AppState> for Auth {
    type Rejection = ServiceError;

    async fn from_request_parts(parts: &mut Parts, _: &AppState) -> Result<Self, Self::Rejection> {
        if let Ok(TypedHeader(Authorization(bearer))) =
            parts.extract::<TypedHeader<Authorization<Bearer>>>().await
        {
            match validate_jwt(bearer.token()) {
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
        .route("/login-guest", post(login_guest))
        .route("/register", post(register))
        .route("/api-token", post(create_api_token))
        .route("/recover", post(recover))
        .route("/reset", post(reset))
        .route("/refresh", post(refresh))
        .route("/logout", post(logout))
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginRequest {
    identifier: String,
    password: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthenticatedUserResponse {
    jwt: String,
    refresh_token: String,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    refresh_token_valid_until: chrono::DateTime<chrono::Utc>,
}

const JWT_DURATION: Duration = Duration::from_mins(15); // short lived access token

async fn login(
    State(app): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthenticatedUserResponse>, ServiceError> {
    let user = match app
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
    let jwt = generate_jwt(user.user_id, JWT_DURATION);

    Ok(Json(AuthenticatedUserResponse {
        jwt,
        refresh_token: user.refresh_token.token,
        refresh_token_valid_until: user.refresh_token.valid_until,
    }))
}

async fn login_guest(
    State(app): State<AppState>,
) -> Result<Json<AuthenticatedUserResponse>, ServiceError> {
    let user = match app.app.authentication_use_case.create_guest_account().await {
        Ok(user_id) => user_id,
        Err(RegisterError::Internal) => {
            return Err(ServiceError::Internal(format!("Failed to login as guest")));
        }
    };
    let jwt = generate_jwt(user.user_id, JWT_DURATION);

    Ok(Json(AuthenticatedUserResponse {
        jwt,
        refresh_token: user.refresh_token.token,
        refresh_token_valid_until: user.refresh_token.valid_until,
    }))
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RefreshRequest {
    user_id: String,
    refresh_token: String,
}

async fn refresh(
    State(app): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<AuthenticatedUserResponse>, ServiceError> {
    let user_id = UserId::try_from(req.user_id.clone())
        .map_err(|_| ServiceError::BadRequest("Invalid user ID".to_string()))?;
    let user = match app
        .app
        .authentication_use_case
        .refresh(user_id, req.refresh_token)
        .await
    {
        Ok(user_id) => user_id,
        Err(RefreshError::InvalidToken) => {
            return Err(ServiceError::Unauthorized(
                "Invalid refresh token".to_string(),
            ));
        }
        Err(RefreshError::Internal) => {
            return Err(ServiceError::Internal(format!("Failed to refresh token")));
        }
    };
    let jwt = generate_jwt(user.user_id, JWT_DURATION);

    Ok(Json(AuthenticatedUserResponse {
        jwt,
        refresh_token: user.refresh_token.token,
        refresh_token_valid_until: user.refresh_token.valid_until,
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
) -> Result<Json<AuthenticatedUserResponse>, ServiceError> {
    let user = match app
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
    let jwt = generate_jwt(user.user_id, JWT_DURATION);

    Ok(Json(AuthenticatedUserResponse {
        jwt,
        refresh_token: user.refresh_token.token,
        refresh_token_valid_until: user.refresh_token.valid_until,
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

async fn logout(State(app): State<AppState>, auth: Auth) -> Result<(), ServiceError> {
    match app.app.authentication_use_case.logout(auth.user_id).await {
        Ok(()) => Ok(()),
        Err(LogoutError::Internal) => Err(ServiceError::Internal("Failed to logout".to_string())),
    }
}

async fn create_api_token(
    auth: Auth,
    Json(req): Json<CreateApiTokenRequest>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    if req.duration == 0 || req.duration > 60 * 60 * 24 * 365 {
        return Err(ServiceError::BadRequest(
            "Duration must be between 1 second and 365 days".to_string(),
        ));
    }
    let cert = generate_jwt(auth.user_id, Duration::from_secs(req.duration));
    Ok(Json(serde_json::json!({ "certificate": cert })))
}

#[derive(serde::Deserialize)]
struct CreateApiTokenRequest {
    duration: u64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

impl Claims {
    pub fn from_token(token: &str) -> Result<Self, ()> {
        let token_data =
            decode::<Claims>(token, &KEYS.decoding, &Validation::default()).map_err(|_| ())?;

        Ok(token_data.claims)
    }
}

struct Keys {
    encoding: EncodingKey,
    decoding: DecodingKey,
}

impl Keys {
    fn new(secret: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
    }
}

static KEYS: LazyLock<Keys> = LazyLock::new(|| {
    let secret = read_or_generate_secret();
    Keys::new(&secret)
});

fn read_or_generate_secret() -> Vec<u8> {
    let secret = std::env::var("TAK_JWT_SECRET").expect("TAK_JWT_SECRET must be set");
    secret.as_bytes().to_vec()
}

fn generate_jwt(user_id: UserId, duration: Duration) -> String {
    let claims = Claims {
        sub: user_id.to_string(),
        exp: (chrono::Utc::now()
            + chrono::Duration::from_std(duration).unwrap_or_else(|_| chrono::Duration::MAX))
        .timestamp() as usize,
    };
    let token = encode(&Header::default(), &claims, &KEYS.encoding).unwrap();
    token
}

pub fn validate_jwt(token: &str) -> Option<UserId> {
    let claims = Claims::from_token(token).ok()?;
    let user_id = UserId::try_from(claims.sub).ok()?;
    Some(user_id)
}
