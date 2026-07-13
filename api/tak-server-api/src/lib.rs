use std::sync::Arc;

use axum::{Router, response::IntoResponse, routing::get};
use tak_player_connection::PlayerConnectionDriver;
use tak_server_app::Application;

pub use auth::ApiAuthPort;
pub use ws::WsService;

pub mod account;
mod auth;
pub mod chat;
pub mod game;
pub mod history;
pub mod matches;
pub mod player;
pub mod profile;
pub mod puzzle;
pub mod seek;
pub mod tournament;
pub mod ws;

#[derive(Clone)]
pub struct AppState {
    pub app: Arc<Application>,
    pub auth: Arc<dyn ApiAuthPort + Send + Sync + 'static>,
    pub connection_driver: Arc<PlayerConnectionDriver>,
    pub ws: Arc<WsService>,
}

pub async fn serve(
    app: Arc<Application>,
    auth: Arc<dyn ApiAuthPort + Send + Sync + 'static>,
    ws: Arc<WsService>,
    connection_driver: Arc<PlayerConnectionDriver>,
    shutdown_signal: impl std::future::Future<Output = ()> + Send + 'static,
) {
    let state = AppState {
        app,
        auth,
        ws,
        connection_driver,
    };

    let router = Router::new()
        .route("/ws", get(ws::ws_handler))
        .nest("/seeks", seek::register_routes())
        .nest("/auth", auth::register_routes())
        .nest("/games", game::register_routes())
        .nest("/puzzles", puzzle::register_routes())
        .nest("/chat", chat::register_routes())
        .nest("/tournaments", tournament::register_routes())
        .nest("/matches", matches::register_routes())
        .nest("/accounts", account::register_routes())
        .nest("/players", player::register_routes())
        .nest("/profiles", profile::register_routes())
        .nest("/history", history::register_routes());

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
pub struct PaginationQuery {
    page: usize,
    page_size: usize,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total_count: u32,
}

#[allow(unused)]
pub enum ServiceError {
    NotFound(String),
    Unauthorized(String),
    BadRequest(String),
    NotPossible(String),
    Internal(String),
    Forbidden(String),
}

impl ServiceError {
    pub fn message(&self) -> &str {
        match self {
            ServiceError::NotFound(msg)
            | ServiceError::Unauthorized(msg)
            | ServiceError::BadRequest(msg)
            | ServiceError::NotPossible(msg)
            | ServiceError::Internal(msg)
            | ServiceError::Forbidden(msg) => msg,
        }
    }

    pub fn status_code(&self) -> axum::http::StatusCode {
        match self {
            ServiceError::NotFound(_) => axum::http::StatusCode::NOT_FOUND,
            ServiceError::Unauthorized(_) => axum::http::StatusCode::UNAUTHORIZED,
            ServiceError::BadRequest(_) => axum::http::StatusCode::BAD_REQUEST,
            ServiceError::NotPossible(_) => axum::http::StatusCode::BAD_REQUEST,
            ServiceError::Internal(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
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
            ServiceError::Internal(msg) => write!(f, "Internal error: {}", msg),
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
