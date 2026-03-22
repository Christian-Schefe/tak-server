use std::sync::Arc;

use axum::{Router, routing::get};
use tak_server_app::{Application, ports::authentication::AuthenticationPort};

use crate::acl::LegacyAPIAntiCorruptionLayer;

mod event;

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
        Router::new().route("/events", get(event::get_all_events)),
    );

    let port = std::env::var("TAK_LEGACY_HTTP_API_PORT")
        .expect("TAK_LEGACY_HTTP_API_PORT must be set")
        .parse::<u16>()
        .expect("TAK_LEGACY_HTTP_API_PORT must be a valid u16");
    let host = std::env::var("TAK_HOST").expect("TAK_HOST must be set");

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", host, port))
        .await
        .unwrap();

    tracing::info!("Legacy API server listening on port {}", port);
    axum::serve(listener, router.with_state(AppState { app, auth, acl }))
        .with_graceful_shutdown(shutdown_signal)
        .await
        .unwrap();

    tracing::info!("Legacy HTTP API shut down gracefully");
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
