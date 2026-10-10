use std::{sync::LazyLock, time::Duration};

use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use tak_server_app::domain::UserId;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    exp: usize,
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

pub fn generate_jwt(user_id: UserId, duration: Duration) -> String {
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
