use std::sync::Arc;

use axum::response::IntoResponse;
use tak_server_domain::{
    ServiceError, app::AppState, game::ArcGameRepository, jwt::ArcJwtService,
    player::ArcPlayerRepository, transport::ArcTransportService,
};

use crate::{jwt::JwtServiceImpl, persistence::games::GameRepositoryImpl};

trait MyIntoResponse {
    fn my_into_response(self) -> axum::http::Response<axum::body::Body>;
}

impl MyIntoResponse for ServiceError {
    fn my_into_response(self) -> axum::http::Response<axum::body::Body> {
        let (status, msg) = match self {
            ServiceError::NotFound(msg) => (axum::http::StatusCode::NOT_FOUND, msg),
            ServiceError::Unauthorized(msg) => (axum::http::StatusCode::UNAUTHORIZED, msg),
            ServiceError::BadRequest(msg) => (axum::http::StatusCode::BAD_REQUEST, msg),
            ServiceError::NotPossible(msg) => (axum::http::StatusCode::BAD_REQUEST, msg),
            ServiceError::Other(msg) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg),
            ServiceError::Internal(msg) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg),
            ServiceError::Forbidden(msg) => (axum::http::StatusCode::FORBIDDEN, msg),
        };
        let body = serde_json::json!({ "error": msg });
        (status, axum::Json(body)).into_response()
    }
}

pub fn construct_app() -> AppState {
    let game_repo: ArcGameRepository = Arc::new(Box::new(GameRepositoryImpl::new()));
    let player_repo: ArcPlayerRepository = Arc::new(Box::new(PlayerRepositoryImpl::new()));
    let transport_service: ArcTransportService = Arc::new(Box::new(TransportServiceImpl::new()));

    let jwt_service: ArcJwtService = Arc::new(Box::new(JwtServiceImpl {}));
    tak_server_domain::app::construct_app(game_repo, player_repo, jwt_service, transport_service)
}
