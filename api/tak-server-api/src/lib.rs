use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Query, State},
    response::IntoResponse,
    routing::{delete, get, post},
};
use tak_player_connection::PlayerConnectionDriver;
use tak_server_api_contract::auth::IdentityInfo;
use tak_server_app::{Application, services::player_resolver::ResolveError};

use crate::auth::StrictAuth;
pub use auth::ApiAuthPort;
pub use ws::WsService;

mod auth;
pub mod chat;
pub mod game;
pub mod matches;
pub mod player;
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

    let admin_router = Router::new().route("/bot-certificate", post(auth::get_bot_certificate));

    let router = Router::new()
        .nest("/admin", admin_router)
        .route("/whoami", get(who_am_i))
        .route("/ws", get(ws::ws_handler))
        .route("/seeks", get(seek::get_seeks))
        .route("/seeks", post(seek::create_seek))
        .route("/seeks/{seek_id}", delete(seek::cancel_seek))
        .route("/seeks/{seek_id}/accept", post(seek::accept_seek))
        .route("/games", get(game::get_games))
        .route("/games/{game_id}", get(game::get_game_status))
        .route("/games/{game_id}/resign", post(game::resign_game))
        .route("/games/{game_id}/request", post(game::set_request))
        .route(
            "/games/{game_id}/request/accept",
            post(game::accept_request),
        )
        .route("/profiles/{account_id}", get(player::get_account_profile))
        .route("/me/profile", post(player::update_account_profile))
        .route(
            "/profiles/{account_id}/picture",
            get(player::get_profile_picture),
        )
        .route("/me/profile/picture", post(player::set_profile_picture))
        .route("/usernames/{username}", get(player::get_player_by_username))
        .route(
            "/accounts/{account_id}",
            get(player::get_player_by_account_id),
        )
        .route("/players/{player_id}", get(player::get_player_info))
        .route("/players/{player_id}/stats", get(player::get_player_stats))
        .route(
            "/players/{player_id}/rating-history",
            get(player::get_rating_history),
        )
        .route("/players/{player_id}/games", get(player::get_games_history));

    let router = puzzle::register_routes(router);
    let router = chat::register_routes(router);
    let router = tournament::register_routes(router);
    let router = matches::register_routes(router);

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

#[derive(serde::Serialize, serde::Deserialize)]
pub struct WhoAmIQueryParams {
    pub bot: Option<bool>,
}

async fn who_am_i(
    auth: StrictAuth,
    State(app): State<AppState>,
    Query(params): Query<WhoAmIQueryParams>,
) -> Result<Json<IdentityInfo>, ServiceError> {
    let new_guest = auth.account.is_none();
    if params.bot.is_some_and(|x| x) && auth.account.as_ref().is_none_or(|acc| !acc.is_bot()) {
        return Err(ServiceError::Unauthorized(
            "Bots must use bot account".to_string(),
        ));
    }
    let account = if let Some(account) = auth.account {
        account
    } else {
        match app.auth.create_guest().await {
            Some(guest_account) => guest_account,
            None => {
                return Err(ServiceError::Internal(
                    "Failed to create guest account".to_string(),
                ));
            }
        }
    };
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
        player_id: player_id.to_string(),
        is_guest: account.is_guest(),
        new_guest,
        jwt: app.auth.generate_account_jwt(
            &account.account_id,
            std::time::Duration::from_secs(60 * 60 * 24),
        ),
    }))
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
