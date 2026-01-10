use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::{IntoResponse, Response},
    routing::get,
};
use futures::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use tak_server_app::{Application, services::player_resolver::ResolveError};

use crate::auth::Auth;
mod auth;
mod rating;
pub use auth::ApiAuthPort;

#[derive(Clone)]
pub struct AppState {
    pub app: Arc<Application>,
    pub auth: Arc<dyn ApiAuthPort + Send + Sync + 'static>,
}

pub async fn serve(
    app: Arc<Application>,
    auth: Arc<dyn ApiAuthPort + Send + Sync + 'static>,
    shutdown_signal: impl std::future::Future<Output = ()> + Send + 'static,
) {
    let state = AppState { app, auth };
    let router = Router::new()
        .route("/whoami", get(who_am_i))
        .route("/guest", get(get_guest))
        .route("/ws", get(ws_handler))
        .route("/ratings/{player_id}", get(rating::get_rating));

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
        username: auth.account.username,
        display_name: auth.account.display_name,
        player_id: player_id.to_string(),
        is_guest: false,
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
        username: String,
        display_name: String,
        player_id: String,
        is_guest: bool,
    },
    Unauthenticated,
}

async fn ws_handler(ws: WebSocketUpgrade, app: State<AppState>) -> Response {
    ws.on_upgrade(move |socket| async move {
        let (ws_sender, ws_receiver) = socket.split();
        let receive_task = tokio::spawn(receive_ws(ws_receiver));
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let send_task = tokio::spawn(send_ws(ws_sender, rx));

        let (receive_res, send_res) = tokio::join!(receive_task, send_task);
        if let Err(e) = receive_res {
            log::error!("WebSocket receive task failed: {}", e);
        }
        if let Err(e) = send_res {
            log::error!("WebSocket send task failed: {}", e);
        }
    })
}

async fn receive_ws(mut ws_receiver: SplitStream<WebSocket>) {
    while let Some(msg) = ws_receiver.next().await {
        match msg {
            Ok(axum::extract::ws::Message::Text(text)) => {
                log::info!("Received WS text message: {}", text);
            }
            Ok(axum::extract::ws::Message::Binary(bin)) => {
                log::info!("Received WS binary message: {:?}", bin);
            }
            Ok(axum::extract::ws::Message::Close(frame)) => {
                log::info!("WS connection closed: {:?}", frame);
                break;
            }
            Err(e) => {
                log::error!("WS error: {}", e);
                break;
            }
            _ => {}
        }
    }
}

async fn send_ws(
    mut ws_sender: SplitSink<WebSocket, Message>,
    channel: tokio::sync::mpsc::UnboundedReceiver<axum::extract::ws::Message>,
) -> Result<(), ServiceError> {
    let mut channel = channel;
    while let Some(msg) = channel.recv().await {
        ws_sender
            .send(msg)
            .await
            .map_err(|e| ServiceError::Internal(format!("Failed to send WS message: {}", e)))?;
    }
    Ok(())
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
