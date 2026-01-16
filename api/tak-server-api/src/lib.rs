use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    response::IntoResponse,
    routing::{delete, get, post},
};
use tak_player_connection::PlayerConnectionDriver;
use tak_server_app::{Application, services::player_resolver::ResolveError};

use crate::auth::Auth;
pub use auth::ApiAuthPort;
pub use ws::WsService;

mod auth;
mod game;
mod player;
mod seek;
mod ws;

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
        .route("/whoami", get(who_am_i))
        .route("/guest", get(get_guest))
        .route("/ws", get(ws::ws_handler))
        .route("/seeks", get(seek::get_seeks))
        .route("/seeks", post(seek::create_seek))
        .route("/seeks/{seek_id}", delete(seek::cancel_seek))
        .route("/seeks/{seek_id}/accept", post(seek::accept_seek))
        .route("/games", get(game::get_games))
        .route("/games/{game_id}", get(game::get_game_status))
        .route("/players/{player_id}", get(player::get_player_info))
        .route("/usernames/{username}", get(player::get_player_by_username))
        .route("/players/{player_id}/stats", get(player::get_player_stats));

    let port = std::env::var("TAK_HTTP_API_PORT")
        .expect("TAK_HTTP_API_PORT must be set")
        .parse::<u16>()
        .expect("TAK_HTTP_API_PORT must be a valid u16");
    let host = std::env::var("TAK_HOST").expect("TAK_HOST must be set");
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", host, port))
        .await
        .unwrap();

    log::info!("API server listening on port {}", port);
    axum::serve(listener, router.with_state(state))
        .with_graceful_shutdown(shutdown_signal)
        .await
        .unwrap();

    log::info!("HTTP API shut down gracefully");
}

async fn get_guest(
    auth: Result<Auth, ServiceError>,
    State(app): State<AppState>,
) -> Result<Json<GuestInfo>, ServiceError> {
    let token = match &auth {
        Ok(auth) => auth.guest_jwt.as_deref(),
        Err(_) => None,
    };
    let guest_jwt = app.auth.generate_or_refresh_guest_jwt(token);

    Ok(Json(GuestInfo { jwt: guest_jwt }))
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuestInfo {
    pub jwt: String,
}

async fn who_am_i(
    auth: Result<Auth, ServiceError>,
    State(app): State<AppState>,
) -> Result<Json<IdentityInfo>, ServiceError> {
    let Ok(auth) = auth else {
        return Ok(Json(IdentityInfo::Unauthenticated));
    };
    let player_id = app
        .app
        .player_resolver_service
        .resolve_player_id_by_account_id(&auth.account.account_id)
        .await
        .map_err(|ResolveError::Internal| {
            ServiceError::Internal("Failed to resolve player ID".to_string())
        })?;

    Ok(Json(IdentityInfo::Authenticated {
        account_id: auth.account.account_id.to_string(),
        player_id: player_id.to_string(),
        is_guest: false,
        ws_jwt: app.auth.generate_account_jwt(&auth.account.account_id),
    }))
}

#[derive(serde::Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum IdentityInfo {
    Authenticated {
        account_id: String,
        player_id: String,
        is_guest: bool,
        ws_jwt: String,
    },
    Unauthenticated,
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
