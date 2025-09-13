mod json;
mod v2;

use std::time::Duration;

use crate::{
    client::ClientId,
    game::{Game, GameId},
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
    GameAction {
        game_id: GameId,
        action: TakAction,
    },
    GameTimeUpdate {
        game_id: GameId,
        remaining: (Duration, Duration),
    },
    GameUndo {
        game_id: GameId,
    },
    GameOver {
        game_id: GameId,
        game_state: TakGameState,
    },
    GameUndoRequest {
        game_id: GameId,
        request: bool,
    },
    GameDrawOffer {
        game_id: GameId,
        offer: bool,
    },
    PlayersOnline {
        players: Vec<String>,
    },
    ObserveGame {
        game: Game,
    },
}

pub fn handle_client_message(protocol: &Protocol, id: &ClientId, msg: String) {
    match protocol {
        Protocol::V2 => v2::handle_client_message(id, msg),
        Protocol::JSON => todo!(),
    }
}

pub fn handle_server_message(protocol: &Protocol, id: &ClientId, msg: &ServerMessage) {
    match protocol {
        Protocol::V2 => v2::handle_server_message(id, msg),
        Protocol::JSON => todo!(),
    }
}
