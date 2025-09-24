mod json;
mod v2;

use std::time::Duration;

use crate::{
    AppState,
    client::ClientId,
    game::GameId,
    player::PlayerUsername,
    seek::SeekId,
    tak::{TakAction, TakGameState},
    util::LazyInit,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Protocol {
    V0,
    V2,
    JSON,
}

impl Protocol {
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "0" => Some(Protocol::V0),
            "2" => Some(Protocol::V2),
            "3" => Some(Protocol::JSON),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum ServerMessage {
    SeekList {
        add: bool,
        seek_id: SeekId,
    },
    GameList {
        add: bool,
        game_id: GameId,
    },
    GameStart {
        game_id: GameId,
    },
    GameMessage {
        game_id: GameId,
        message: ServerGameMessage,
    },
    PlayersOnline {
        players: Vec<String>,
    },
    ChatMessage {
        from: PlayerUsername,
        message: String,
        source: ChatMessageSource,
    },
    RoomMembership {
        room: String,
        joined: bool,
    },
    AcceptRematch {
        seek_id: SeekId,
    },
}

#[derive(Clone, Debug)]
pub enum ServerGameMessage {
    Action(TakAction),
    TimeUpdate { remaining: (Duration, Duration) },
    Undo,
    GameOver(TakGameState),
    UndoRequest { request: bool },
    DrawOffer { offer: bool },
}

#[derive(Clone, Debug)]
pub enum ChatMessageSource {
    Global,
    Room(String),
    Private,
}

pub trait ProtocolService {
    fn init(&self, client_service: &AppState);
    fn handle_client_message(&self, protocol: &Protocol, id: &ClientId, msg: String);
    fn handle_server_message(&self, protocol: &Protocol, id: &ClientId, msg: &ServerMessage);
    fn on_authenticated(&self, protocol: &Protocol, id: &ClientId, username: &PlayerUsername);
    fn register_http_endpoints(&self, router: axum::Router<AppState>) -> axum::Router<AppState>;
}

pub struct ProtocolServiceImpl {
    v2: LazyInit<v2::ProtocolV2Handler>,
    json: LazyInit<json::ProtocolJsonHandler>,
}

impl ProtocolServiceImpl {
    pub fn new() -> Self {
        Self {
            v2: LazyInit::new(),
            json: LazyInit::new(),
        }
    }
}

impl ProtocolService for ProtocolServiceImpl {
    fn init(&self, app: &AppState) {
        let _ = self.v2.init(v2::ProtocolV2Handler::new(
            app.client_service.clone(),
            app.seek_service.clone(),
            app.player_service.clone(),
            app.chat_service.clone(),
            app.game_service.clone(),
        ));
        let _ = self.json.init(json::ProtocolJsonHandler::new(
            app.client_service.clone(),
            app.player_service.clone(),
        ));
    }

    fn handle_client_message(&self, protocol: &Protocol, id: &ClientId, msg: String) {
        match protocol {
            Protocol::V0 | Protocol::V2 => self.v2.get().handle_client_message(id, msg),
            Protocol::JSON => self.json.get().handle_client_message(id, msg),
        }
    }

    fn handle_server_message(&self, protocol: &Protocol, id: &ClientId, msg: &ServerMessage) {
        match protocol {
            Protocol::V0 | Protocol::V2 => self.v2.get().handle_server_message(id, msg),
            Protocol::JSON => self.json.get().handle_server_message(id, msg),
        }
    }

    fn on_authenticated(&self, protocol: &Protocol, id: &ClientId, username: &PlayerUsername) {
        match protocol {
            Protocol::V0 | Protocol::V2 => self.v2.get().on_authenticated(id, username),
            Protocol::JSON => {}
        }
    }

    fn register_http_endpoints(&self, router: axum::Router<AppState>) -> axum::Router<AppState> {
        router.nest("/v3", json::register_http_endpoints())
    }
}
