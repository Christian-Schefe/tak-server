use std::sync::Arc;

use crate::{ServiceResult, player::PlayerUsername};

pub type ArcJwtService = Arc<Box<dyn JwtService + Send + Sync>>;
pub trait JwtService {
    fn validate_jwt(&self, token: &str) -> ServiceResult<PlayerUsername>;
}
