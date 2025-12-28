use axum::response::IntoResponse;

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
