use std::sync::Arc;

use axum::response::IntoResponse;
use tak_persistence::sqlite::{games::SqliteGameRepository, players::SqlitePlayerRepository};
use tak_server_domain::{
    ServiceError,
    app::{LazyAppState, construct_app},
    game::ArcGameRepository,
    jwt::ArcJwtService,
    player::ArcPlayerRepository,
    transport::ArcTransportService,
};

use crate::{client::TransportServiceImpl, jwt::JwtServiceImpl};

pub struct MyServiceError(ServiceError);

impl IntoResponse for MyServiceError {
    fn into_response(self) -> axum::http::Response<axum::body::Body> {
        let (status, msg) = match self.0 {
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

impl From<ServiceError> for MyServiceError {
    fn from(value: ServiceError) -> Self {
        MyServiceError(value)
    }
}

pub async fn run() {
    let app = LazyAppState::new();
    let transport_service_impl = TransportServiceImpl::new(app.clone());

    let game_repo: ArcGameRepository = Arc::new(Box::new(SqliteGameRepository::new()));
    let player_repo: ArcPlayerRepository = Arc::new(Box::new(SqlitePlayerRepository::new()));
    let transport_service: ArcTransportService = Arc::new(Box::new(transport_service_impl.clone()));

    let jwt_service: ArcJwtService = Arc::new(Box::new(JwtServiceImpl {}));
    construct_app(
        app.clone(),
        game_repo,
        player_repo,
        jwt_service,
        transport_service,
    );

    transport_service_impl.run(app.clone()).await;
}
