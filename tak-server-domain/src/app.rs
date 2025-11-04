use std::sync::{Arc, OnceLock};

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
pub struct LazyAppState(Arc<OnceLock<AppState>>);

impl LazyAppState {
    pub fn new() -> Self {
        Self(Arc::new(OnceLock::new()))
    }
    pub fn unwrap(&self) -> &AppState {
        self.0.get().expect("AppState not initialized")
    }
    pub fn transport_service(&self) -> ArcTransportService {
        self.unwrap().transport_service.clone()
    }
    pub fn game_service(&self) -> ArcGameService {
        self.unwrap().game_service.clone()
    }
    pub fn chat_service(&self) -> ArcChatService {
        self.unwrap().chat_service.clone()
    }
    pub fn seek_service(&self) -> ArcSeekService {
        self.unwrap().seek_service.clone()
    }
    pub fn player_service(&self) -> ArcPlayerService {
        self.unwrap().player_service.clone()
    }
    pub fn email_service(&self) -> ArcEmailService {
        self.unwrap().email_service.clone()
    }
    pub fn jwt_service(&self) -> ArcJwtService {
        self.unwrap().jwt_service.clone()
    }
    pub fn player_connection_service(&self) -> ArcPlayerConnectionService {
        self.unwrap().player_connection_service.clone()
    }
}

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

impl AppState {
    pub async fn start(&self) {
        self.player_service
            .load_unique_usernames()
            .await
            .expect("Failed to load unique usernames");
    }
}

pub fn construct_app(
    lazy_app_state: LazyAppState,
    game_repository: ArcGameRepository,
    player_repository: ArcPlayerRepository,
    jwt_service: ArcJwtService,
    transport_service: ArcTransportService,
) {
    let player_connection_service: ArcPlayerConnectionService = Arc::new(Box::new(
        PlayerConnectionServiceImpl::new(lazy_app_state.clone()),
    ));

    let email_service: ArcEmailService = Arc::new(Box::new(EmailServiceImpl {}));

    let player_service: ArcPlayerService = Arc::new(Box::new(PlayerServiceImpl::new(
        transport_service.clone(),
        email_service.clone(),
        jwt_service.clone(),
        player_repository.clone(),
    )));

    let game_service: ArcGameService = Arc::new(Box::new(GameServiceImpl::new(
        transport_service.clone(),
        player_connection_service.clone(),
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

    lazy_app_state.0.set(app.clone()).ok().unwrap();
}
