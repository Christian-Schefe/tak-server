use std::sync::LazyLock;

use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use tak_server_app::domain::AccountId;

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

pub fn generate_jwt(account_id: &AccountId) -> String {
    let claims = Claims {
        sub: account_id.to_string(),
        exp: (chrono::Utc::now() + chrono::Duration::hours(24)).timestamp() as usize,
    };
    let token = encode(&Header::default(), &claims, &KEYS.encoding).unwrap();
    token
}
