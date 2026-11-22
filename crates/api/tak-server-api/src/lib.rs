use std::sync::Arc;

use axum::{Router, response::IntoResponse, routing::get};
use tak_player_connection::PlayerConnectionDriver;
use tak_server_app::{
    Application,
    errors::{
        GameNotFound, InternalError, InvalidCredentials, MatchNotFound, OneOf, OneOfNarrow,
        TypeSet, UserNotFound,
    },
};

use tower_http::csrf::CsrfLayer;
pub use ws::WsService;

mod auth;
mod chat;
mod game;
mod history;
mod matches;
mod player;
mod profile;
mod seek;
mod tournament;
mod ws;

#[derive(Clone)]
pub struct AppState {
    pub app: Arc<Application>,
    pub connection_driver: Arc<PlayerConnectionDriver>,
    pub ws: Arc<WsService>,
}

pub async fn serve(
    app: Arc<Application>,
    ws: Arc<WsService>,
    connection_driver: Arc<PlayerConnectionDriver>,
    shutdown_signal: impl std::future::Future<Output = ()> + Send + 'static,
) {
    let state = AppState {
        app,
        ws,
        connection_driver,
    };

    let csrf_layer = CsrfLayer::new();

    let router = Router::new()
        .route("/ws", get(ws::ws_handler))
        .nest("/seeks", seek::register_routes())
        .nest("/auth", auth::register_routes())
        .nest("/games", game::register_routes())
        .nest("/chat", chat::register_routes())
        .nest("/tournaments", tournament::register_routes())
        .nest("/matches", matches::register_routes())
        .nest("/players", player::register_routes())
        .nest("/profiles", profile::register_routes())
        .nest("/history", history::register_routes())
        .layer(csrf_layer);

    let port = std::env::var("TAK_HTTP_API_PORT")
        .expect("TAK_HTTP_API_PORT must be set")
        .parse::<u16>()
        .expect("TAK_HTTP_API_PORT must be a valid u16");
    let host = std::env::var("TAK_HOST").expect("TAK_HOST must be set");
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", host, port))
        .await
        .unwrap();

    tracing::info!("API server listening on port {}", port);
    axum::serve(listener, router.with_state(state))
        .with_graceful_shutdown(shutdown_signal)
        .await
        .unwrap();

    tracing::info!("HTTP API shut down gracefully");
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PaginationQuery {
    page: usize,
    page_size: usize,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PaginatedResponse<T> {
    items: Vec<T>,
    total_count: u32,
}

#[allow(unused)]
pub enum ServiceError {
    NotFound(String),
    Unauthorized(String),
    BadRequest(String),
    NotPossible(String),
    Internal,
    Forbidden(String),
}

impl ServiceError {
    fn message(&self) -> &str {
        match self {
            ServiceError::NotFound(msg)
            | ServiceError::Unauthorized(msg)
            | ServiceError::BadRequest(msg)
            | ServiceError::NotPossible(msg)
            | ServiceError::Forbidden(msg) => msg,
            ServiceError::Internal => "Internal server error",
        }
    }

    fn status_code(&self) -> axum::http::StatusCode {
        match self {
            ServiceError::NotFound(_) => axum::http::StatusCode::NOT_FOUND,
            ServiceError::Unauthorized(_) => axum::http::StatusCode::UNAUTHORIZED,
            ServiceError::BadRequest(_) => axum::http::StatusCode::BAD_REQUEST,
            ServiceError::NotPossible(_) => axum::http::StatusCode::BAD_REQUEST,
            ServiceError::Internal => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            ServiceError::Forbidden(_) => axum::http::StatusCode::FORBIDDEN,
        }
    }
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceError::NotFound(msg) => write!(f, "Not found: {}", msg),
            ServiceError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            ServiceError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            ServiceError::NotPossible(msg) => write!(f, "Not possible: {}", msg),
            ServiceError::Internal => write!(f, "Internal error"),
            ServiceError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
        }
    }
}

impl IntoResponse for ServiceError {
    fn into_response(self) -> axum::http::Response<axum::body::Body> {
        let status = self.status_code();
        let msg = self.message().to_string();
        let body = serde_json::json!({ "error": msg });
        (status, axum::Json(body)).into_response()
    }
}

fn handle_error<E: TypeSet, A: IntoServiceError + 'static, I>(
    err: OneOf<E>,
) -> Result<ServiceError, OneOf<<E::Coprod as OneOfNarrow<A, I>>::Remainder>>
where
    E::Coprod: OneOfNarrow<A, I>,
{
    match err.narrow() {
        Ok(x) => Ok(x.into_service_error()),
        Err(e) => Err(e),
    }
}

trait IntoServiceError {
    fn into_service_error(self) -> ServiceError;
}

impl IntoServiceError for InternalError {
    fn into_service_error(self) -> ServiceError {
        ServiceError::Internal
    }
}

impl IntoServiceError for UserNotFound {
    fn into_service_error(self) -> ServiceError {
        ServiceError::NotFound("User not found".to_string())
    }
}

impl IntoServiceError for GameNotFound {
    fn into_service_error(self) -> ServiceError {
        ServiceError::NotFound("Game not found".to_string())
    }
}

impl IntoServiceError for MatchNotFound {
    fn into_service_error(self) -> ServiceError {
        ServiceError::NotFound("Match not found".to_string())
    }
}

impl IntoServiceError for InvalidCredentials {
    fn into_service_error(self) -> ServiceError {
        ServiceError::Unauthorized("Invalid credentials".to_string())
    }
}
