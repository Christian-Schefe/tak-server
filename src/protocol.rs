mod json;
mod v2;

use std::time::Duration;

use crate::{
    client::ClientId,
    game::{Game, GameId},
    player::PlayerUsername,
    seek::Seek,
    tak::{TakAction, TakGameState},
};

pub enum Protocol {
    V2,
    JSON,
}

impl Protocol {
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
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
        seek: Seek,
    },
    GameList {
        add: bool,
        game: Game,
    },
    GameStart {
        game: Game,
    },
    GameMessage {
        game_id: GameId,
        message: ServerGameMessage,
    },
    PlayersOnline {
        players: Vec<String>,
    },
    ObserveGame {
        game: Game,
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

pub fn handle_client_message(protocol: &Protocol, id: &ClientId, msg: String) {
    match protocol {
        Protocol::V2 => v2::handle_client_message(id, msg),
        Protocol::JSON => json::handle_client_message(id, msg),
    }
}

pub fn handle_server_message(protocol: &Protocol, id: &ClientId, msg: &ServerMessage) {
    match protocol {
        Protocol::V2 => v2::handle_server_message(id, msg),
        Protocol::JSON => json::handle_server_message(id, msg),
    }
}

pub fn on_authenticated(protocol: &Protocol, id: &ClientId, username: &PlayerUsername) {
    match protocol {
        Protocol::V2 => v2::on_authenticated(id, username),
        Protocol::JSON => {}
    }
}

pub fn register_http_endpoints(router: axum::Router) -> axum::Router {
    router.nest("/v3", json::register_http_endpoints())
}
