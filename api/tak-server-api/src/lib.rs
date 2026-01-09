use std::sync::Arc;

use axum::{Json, Router, extract::State, response::IntoResponse, routing::get};
use tak_server_app::{Application, services::player_resolver::ResolveError};

use crate::auth::Auth;
mod auth;
pub use auth::ApiAuthPort;

#[derive(Clone)]
pub struct AppState {
    pub app: Arc<Application>,
    pub auth: Arc<dyn ApiAuthPort + Send + Sync + 'static>,
}

pub async fn serve(app: Arc<Application>, auth: Arc<dyn ApiAuthPort + Send + Sync + 'static>) {
    let state = AppState { app, auth };
    let router = Router::new().route("/whoami", get(who_am_i));

    let port = std::env::var("TAK_HTTP_API_PORT")
        .expect("TAK_HTTP_API_PORT must be set")
        .parse::<u16>()
        .expect("TAK_HTTP_API_PORT must be a valid u16");
    let host = std::env::var("TAK_HOST").expect("TAK_HOST must be set");
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", host, port))
        .await
        .unwrap();

    axum::serve(listener, router.with_state(state))
        .await
        .unwrap();
}

async fn who_am_i(
    Auth(account): Auth,
    State(app): State<AppState>,
) -> Result<Json<IdentityInfo>, ServiceError> {
    let player_id = app
        .app
        .player_resolver_service
        .resolve_player_id_by_account_id(&account.account_id)
        .await
        .map_err(|ResolveError::Internal| {
            ServiceError::Internal("Failed to resolve player ID".to_string())
        })?;

    Ok(Json(IdentityInfo {
        account_id: account.account_id.to_string(),
        username: account.username,
        display_name: account.display_name,
        player_id: player_id.to_string(),
        is_guest: false,
    }))
}

#[derive(serde::Serialize)]
pub struct IdentityInfo {
    pub account_id: String,
    pub username: String,
    pub display_name: String,
    pub player_id: String,
    pub is_guest: bool,
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
        let (status, msg) = match self {
            ServiceError::NotFound(msg) => (axum::http::StatusCode::NOT_FOUND, msg),
            ServiceError::Unauthorized(msg) => (axum::http::StatusCode::UNAUTHORIZED, msg),
            ServiceError::BadRequest(msg) => (axum::http::StatusCode::BAD_REQUEST, msg),
            ServiceError::NotPossible(msg) => (axum::http::StatusCode::BAD_REQUEST, msg),
            ServiceError::Internal(msg) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg),
            ServiceError::Forbidden(msg) => (axum::http::StatusCode::FORBIDDEN, msg),
        };
        let body = serde_json::json!({ "error": msg });
        (status, axum::Json(body)).into_response()
    }
}
