use std::sync::LazyLock;

use axum::{
    Json, RequestPartsExt, debug_handler,
    extract::{FromRequestParts, State},
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tak_server_domain::{ServiceError, jwt::JwtService, player::PlayerUsername};
use uuid::Uuid;

use crate::AppState;
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    exp: usize,
}

#[derive(Debug)]
pub enum AuthError {
    BadRequest,
    WrongCredentials,
    InvalidToken,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AuthError::WrongCredentials => (StatusCode::UNAUTHORIZED, "Wrong credentials"),
            AuthError::InvalidToken => (StatusCode::BAD_REQUEST, "Invalid token"),
            AuthError::BadRequest => (StatusCode::BAD_REQUEST, "Bad request"),
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}

impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract the token from the authorization header
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| AuthError::InvalidToken)?;
        // Decode the user data
        let token_data = decode::<Claims>(bearer.token(), &KEYS.decoding, &Validation::default())
            .map_err(|_| AuthError::InvalidToken)?;

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
    if let Ok(secret) = std::env::var("TAK_JWT_SECRET") {
        secret.as_bytes().to_vec()
    } else {
        println!("JWT secret not found, generating a random one...");
        Uuid::new_v4().as_bytes().to_vec()
    }
}

pub fn generate_jwt(username: &PlayerUsername) -> Result<String, AuthError> {
    let claims = Claims {
        sub: username.clone(),
        exp: (chrono::Utc::now() + chrono::Duration::hours(24)).timestamp() as usize,
    };
    let token =
        encode(&Header::default(), &claims, &KEYS.encoding).map_err(|_| AuthError::BadRequest)?;
    Ok(token)
}

#[derive(Deserialize)]
pub struct AuthPayload {
    pub username: PlayerUsername,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthBody {
    pub token: String,
}

#[debug_handler]
pub async fn handle_login(
    State(app): State<AppState>,
    Json(payload): Json<AuthPayload>,
) -> Result<Json<AuthBody>, AuthError> {
    app.player_service
        .validate_login(&payload.username, &payload.password)
        .map_err(|_| AuthError::WrongCredentials)?;
    let token = generate_jwt(&payload.username)?;
    Ok(Json(AuthBody { token }))
}

pub struct JwtServiceImpl {}

impl JwtService for JwtServiceImpl {
    fn validate_jwt(&self, token: &str) -> tak_server_domain::ServiceResult<PlayerUsername> {
        match decode::<Claims>(token, &KEYS.decoding, &Validation::default()) {
            Ok(data) => Ok(data.claims.sub),
            Err(_) => ServiceError::unauthorized("Invalid token"),
        }
    }
}
