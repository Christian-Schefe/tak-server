use std::sync::Arc;

use axum::response::IntoResponse;
use thiserror::Error;

use crate::{
    chat::{ChatService, ChatServiceImpl},
    client::{ClientService, ClientServiceImpl},
    email::{EmailService, EmailServiceImpl},
    game::{GameService, GameServiceImpl},
    persistence::{
        games::{GameRepository, GameRepositoryImpl},
        players::{PlayerRepository, PlayerRepositoryImpl},
    },
    player::{PlayerService, PlayerServiceImpl},
    protocol::{ProtocolService, ProtocolServiceImpl},
    seek::{SeekService, SeekServiceImpl},
};

pub type ArcClientService = Arc<Box<dyn ClientService + Send + Sync + 'static>>;
pub type ArcGameService = Arc<Box<dyn GameService + Send + Sync + 'static>>;
pub type ArcSeekService = Arc<Box<dyn SeekService + Send + Sync + 'static>>;
pub type ArcPlayerService = Arc<Box<dyn PlayerService + Send + Sync + 'static>>;
pub type ArcEmailService = Arc<Box<dyn EmailService + Send + Sync + 'static>>;
pub type ArcProtocolService = Arc<Box<dyn ProtocolService + Send + Sync + 'static>>;
pub type ArcChatService = Arc<Box<dyn ChatService + Send + Sync + 'static>>;

pub type ArcPlayerRepository = Arc<Box<dyn PlayerRepository + Send + Sync + 'static>>;
pub type ArcGameRepository = Arc<Box<dyn GameRepository + Send + Sync + 'static>>;

#[derive(Clone)]
pub struct AppState {
    pub client_service: ArcClientService,
    pub game_service: ArcGameService,
    pub seek_service: ArcSeekService,
    pub player_service: ArcPlayerService,
    pub email_service: ArcEmailService,
    pub protocol_service: ArcProtocolService,
    pub chat_service: ArcChatService,

    pub player_repository: ArcPlayerRepository,
    pub game_repository: ArcGameRepository,
}

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("unauthorized: {0}")]
    Unauthorized(String),

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("database error: {0}")]
    Database(#[from] DatabaseError),

    #[error("operation not possible: {0}")]
    NotPossible(String),

    #[error("forbidden: {0}")]
    Forbidden(String),

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

impl IntoResponse for ServiceError {
    fn into_response(self) -> axum::http::Response<axum::body::Body> {
        let (status, msg) = match self {
            ServiceError::NotFound(msg) => (axum::http::StatusCode::NOT_FOUND, msg),
            ServiceError::Unauthorized(msg) => (axum::http::StatusCode::UNAUTHORIZED, msg),
            ServiceError::BadRequest(msg) => (axum::http::StatusCode::BAD_REQUEST, msg),
            ServiceError::Database(_) => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            ),
            ServiceError::NotPossible(msg) => (axum::http::StatusCode::BAD_REQUEST, msg),
            ServiceError::Other(msg) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg),
            ServiceError::Internal(msg) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg),
            ServiceError::Forbidden(msg) => (axum::http::StatusCode::FORBIDDEN, msg),
        };
        let body = serde_json::json!({ "error": msg });
        (status, axum::Json(body)).into_response()
    }
}

pub type ServiceResult<T> = Result<T, ServiceError>;

pub fn construct_app() -> AppState {
    let player_repository: ArcPlayerRepository = Arc::new(Box::new(PlayerRepositoryImpl::new()));
    let game_repository: ArcGameRepository = Arc::new(Box::new(GameRepositoryImpl::new()));

    let protocol_service: Arc<Box<dyn ProtocolService + Send + Sync>> =
        Arc::new(Box::new(ProtocolServiceImpl::new()));

    let client_service: Arc<Box<dyn ClientService + Send + Sync>> =
        Arc::new(Box::new(ClientServiceImpl::new(protocol_service.clone())));

    let email_service: Arc<Box<dyn EmailService + Send + Sync>> =
        Arc::new(Box::new(EmailServiceImpl {}));

    let player_service: Arc<Box<dyn PlayerService + Send + Sync>> =
        Arc::new(Box::new(PlayerServiceImpl::new(
            client_service.clone(),
            email_service.clone(),
            player_repository.clone(),
        )));

    let chat_service: Arc<Box<dyn ChatService + Send + Sync>> = Arc::new(Box::new(
        ChatServiceImpl::new(client_service.clone(), player_service.clone()),
    ));

    let game_service: Arc<Box<dyn GameService + Send + Sync>> =
        Arc::new(Box::new(GameServiceImpl::new(
            client_service.clone(),
            player_service.clone(),
            game_repository.clone(),
        )));

    let seek_service: Arc<Box<dyn SeekService + Send + Sync>> = Arc::new(Box::new(
        SeekServiceImpl::new(client_service.clone(), game_service.clone()),
    ));

    let app = AppState {
        client_service,
        game_service,
        seek_service,
        player_service,
        email_service,
        chat_service,
        protocol_service: protocol_service.clone(),

        player_repository,
        game_repository,
    };

    protocol_service.init(&app);

    app
}
