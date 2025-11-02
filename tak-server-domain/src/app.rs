use std::sync::Arc;

use crate::{
    chat::{ArcChatService, ChatServiceImpl},
    email::{ArcEmailService, EmailServiceImpl},
    game::{ArcGameRepository, ArcGameService, GameServiceImpl},
    jwt::ArcJwtService,
    player::{ArcPlayerRepository, ArcPlayerService, PlayerServiceImpl},
    seek::{ArcSeekService, SeekServiceImpl},
    transport::{ArcPlayerConnectionService, ArcTransportService, PlayerConnectionServiceImpl},
};

#[derive(Clone)]
pub struct AppState {
    pub transport_service: ArcTransportService,
    pub game_service: ArcGameService,
    pub seek_service: ArcSeekService,
    pub player_service: ArcPlayerService,
    pub email_service: ArcEmailService,
    pub chat_service: ArcChatService,
    pub jwt_service: ArcJwtService,
    pub player_connection_service: ArcPlayerConnectionService,

    pub game_repository: ArcGameRepository,
    pub player_repository: ArcPlayerRepository,
}

pub fn construct_app(
    game_repository: ArcGameRepository,
    player_repository: ArcPlayerRepository,
    jwt_service: ArcJwtService,
    transport_service: ArcTransportService,
) -> AppState {
    let email_service: ArcEmailService = Arc::new(Box::new(EmailServiceImpl {}));

    let player_service: ArcPlayerService = Arc::new(Box::new(PlayerServiceImpl::new(
        transport_service.clone(),
        email_service.clone(),
        jwt_service.clone(),
        player_repository.clone(),
    )));

    let game_service: ArcGameService = Arc::new(Box::new(GameServiceImpl::new(
        transport_service.clone(),
        player_service.clone(),
        game_repository.clone(),
    )));

    let seek_service: ArcSeekService = Arc::new(Box::new(SeekServiceImpl::new(
        transport_service.clone(),
        game_service.clone(),
    )));

    let chat_service: ArcChatService = Arc::new(Box::new(ChatServiceImpl::new(
        transport_service.clone(),
        player_service.clone(),
    )));

    let player_connection_service: ArcPlayerConnectionService =
        Arc::new(Box::new(PlayerConnectionServiceImpl::new(
            seek_service.clone(),
            game_service.clone(),
            chat_service.clone(),
            transport_service.clone(),
        )));

    let app = AppState {
        transport_service,
        game_service,
        seek_service,
        player_service,
        email_service,
        chat_service,
        jwt_service,
        player_connection_service,

        game_repository,
        player_repository,
    };

    app
}
