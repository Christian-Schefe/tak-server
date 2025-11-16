use axum::response::IntoResponse;
use tak_server_domain::ServiceError;

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
