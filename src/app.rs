use std::sync::Arc;

use thiserror::Error;

use crate::{
    client::{ClientService, ClientServiceImpl},
    email::{EmailService, EmailServiceImpl},
    game::{GameService, GameServiceImpl},
    player::{PlayerService, PlayerServiceImpl},
    seek::{SeekService, SeekServiceImpl},
};

#[derive(Clone)]
pub struct AppState {
    pub client_service: Arc<Box<dyn ClientService + Send + Sync>>,
    pub game_service: Arc<Box<dyn GameService + Send + Sync>>,
    pub seek_service: Arc<Box<dyn SeekService + Send + Sync>>,
    pub player_service: Arc<Box<dyn PlayerService + Send + Sync>>,
    pub email_service: Arc<Box<dyn EmailService + Send + Sync>>,
}

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("unauthorized: {0}")]
    Unauthorized(String),

    #[error("validation failed: {0}")]
    Validation(String),

    #[error("database error: {0}")]
    Database(#[from] DatabaseError),

    #[error("operation not possible: {0}")]
    NotPossible(String),

    #[error("unexpected error: {0}")]
    Other(String),

    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("connection error: {0}")]
    ConnectionError(r2d2::Error),
    #[error("query error: {0}")]
    QueryError(rusqlite::Error),
}

impl ServiceError {
    pub fn validation_err<T, R>(msg: T) -> ServiceResult<R>
    where
        T: Into<String>,
    {
        Err(ServiceError::Validation(msg.into()))
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
}

pub type ServiceResult<T> = Result<T, ServiceError>;

pub fn construct_app() -> AppState {
    let client_service: Arc<Box<dyn ClientService + Send + Sync>> =
        Arc::new(Box::new(ClientServiceImpl::new()));

    let game_service: Arc<Box<dyn GameService + Send + Sync>> =
        Arc::new(Box::new(GameServiceImpl::new(client_service.clone())));

    let seek_service: Arc<Box<dyn SeekService + Send + Sync>> = Arc::new(Box::new(
        SeekServiceImpl::new(client_service.clone(), game_service.clone()),
    ));

    let email_service: Arc<Box<dyn EmailService + Send + Sync>> =
        Arc::new(Box::new(EmailServiceImpl {}));

    let player_service: Arc<Box<dyn PlayerService + Send + Sync>> = Arc::new(Box::new(
        PlayerServiceImpl::new(client_service.clone(), email_service.clone()),
    ));

    AppState {
        client_service,
        game_service,
        seek_service,
        player_service,
        email_service,
    }
}
