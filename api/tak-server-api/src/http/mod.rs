use std::sync::Arc;

use axum::{Router, routing::get};
use log::info;
use tak_server_app::{Application, ports::authentication::AuthenticationPort};

use crate::acl::LegacyAPIAntiCorruptionLayer;

mod event;
mod games_history;
mod rating;

#[derive(Clone)]
pub struct AppState {
    pub app: Arc<Application>,
    pub auth: Arc<dyn AuthenticationPort + Send + Sync + 'static>,
    pub acl: Arc<LegacyAPIAntiCorruptionLayer>,
}

pub async fn run(
    app: Arc<Application>,
    auth: Arc<dyn AuthenticationPort + Send + Sync + 'static>,
    acl: Arc<LegacyAPIAntiCorruptionLayer>,
    shutdown_signal: impl std::future::Future<Output = ()> + Send + 'static,
) {
    let router: Router<AppState> = Router::new().nest(
        "/v1",
        Router::new()
            .route("/games-history", get(games_history::get_all))
            .route("/games-history/{id}", get(games_history::get_by_id))
            .route("/games-history/ptn/{id}", get(games_history::get_ptn_by_id))
            .route("/events", get(event::get_all_events))
            .route("/ratings", get(rating::get_ratings))
            .route("/ratings/{name}", get(rating::get_rating_by_name)),
    );

    let port = std::env::var("TAK_HTTP_API_PORT")
        .expect("TAK_HTTP_API_PORT must be set")
        .parse::<u16>()
        .expect("TAK_HTTP_API_PORT must be a valid u16");

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .unwrap();

    info!("API server listening on port {}", port);
    axum::serve(listener, router.with_state(AppState { app, auth, acl }))
        .with_graceful_shutdown(shutdown_signal)
        .await
        .unwrap();

    info!("HTTP API shut down gracefully");
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedResponse<T> {
    items: Vec<T>,
    total: usize,
    page: usize,
    per_page: usize,
    total_pages: usize,
}
