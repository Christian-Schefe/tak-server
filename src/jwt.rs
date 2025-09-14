use std::sync::LazyLock;

use axum::Json;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::player::{PlayerUsername, validate_login};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
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

pub fn generate_jwt(username: &PlayerUsername) -> String {
    let claims = Claims {
        sub: username.clone(),
        exp: (chrono::Utc::now() + chrono::Duration::hours(24)).timestamp() as usize,
    };
    let token = encode(&Header::default(), &claims, &KEYS.encoding).unwrap();
    token
}

pub fn validate_jwt(token: &str) -> Result<PlayerUsername, String> {
    match decode::<Claims>(token, &KEYS.decoding, &Validation::default()) {
        Ok(data) => Ok(data.claims.sub),
        Err(_) => Err("Invalid token".to_string()),
    }
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

#[axum::debug_handler]
pub async fn handle_login(Json(payload): Json<AuthPayload>) -> Result<Json<AuthBody>, String> {
    validate_login(&payload.username, &payload.password)?;
    let token = generate_jwt(&payload.username);
    Ok(Json(AuthBody { token }))
}
