use thiserror::Error;

pub mod app;
pub mod chat;
mod email;
pub mod game;
pub mod game_history;
pub mod jwt;
pub mod player;
pub mod rating;
pub mod seek;
pub mod transport;
pub mod util;
pub mod event;

#[derive(Debug, Clone, Error)]
pub enum ServiceError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("unauthorized: {0}")]
    Unauthorized(String),

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("operation not possible: {0}")]
    NotPossible(String),

    #[error("forbidden: {0}")]
    Forbidden(String),

    #[error("unexpected error: {0}")]
    Other(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl ServiceError {
    pub fn bad_request<T, R>(msg: T) -> ServiceResult<R>
    where
        T: Into<String>,
    {
        Err(ServiceError::BadRequest(msg.into()))
    }

    pub fn unauthorized<T, R>(msg: T) -> ServiceResult<R>
    where
        T: Into<String>,
    {
        Err(ServiceError::Unauthorized(msg.into()))
    }

    pub fn not_found<T, R>(msg: T) -> ServiceResult<R>
    where
        T: Into<String>,
    {
        Err(ServiceError::NotFound(msg.into()))
    }

    pub fn not_possible<T, R>(msg: T) -> ServiceResult<R>
    where
        T: Into<String>,
    {
        Err(ServiceError::NotPossible(msg.into()))
    }

    pub fn internal<T, R>(msg: T) -> ServiceResult<R>
    where
        T: Into<String>,
    {
        Err(ServiceError::Internal(msg.into()))
    }

    pub fn forbidden<T, R>(msg: T) -> ServiceResult<R>
    where
        T: Into<String>,
    {
        Err(ServiceError::Forbidden(msg.into()))
    }
}

pub type ServiceResult<T> = Result<T, ServiceError>;
