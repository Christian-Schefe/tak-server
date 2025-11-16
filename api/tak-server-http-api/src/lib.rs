use axum::{Router, response::IntoResponse, routing::get};
use log::info;
use tak_server_domain::{ServiceError, app::LazyAppState};

mod event;
mod games_history;
mod rating;

pub async fn run(
    app: LazyAppState,
    shutdown_signal: impl std::future::Future<Output = ()> + Send + 'static,
) {
    let router = Router::new()
        .route("/games-history", get(games_history::get_all))
        .route("/games-history/{id}", get(games_history::get_by_id))
        .route("/games-history/ptn/{id}", get(games_history::get_ptn_by_id))
        .route("/events", get(event::get_all_events))
        .route("/ratings", get(rating::get_ratings))
        .route("/rating/{name}", get(rating::get_rating_by_name));

    let port = std::env::var("TAK_HTTP_API_PORT")
        .unwrap_or_else(|_| "3004".to_string())
        .parse::<u16>()
        .expect("TAK_HTTP_API_PORT must be a valid u16");

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .unwrap();

    info!("API server listening on port {}", port);
    axum::serve(listener, router.with_state(app.unwrap().clone()))
        .with_graceful_shutdown(shutdown_signal)
        .await
        .unwrap();

    info!("HTTP API shut down gracefully");
}

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

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedResponse<T> {
    items: Vec<T>,
    total: usize,
    page: usize,
    per_page: usize,
    total_pages: usize,
}
