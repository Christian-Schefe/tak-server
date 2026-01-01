use std::time::Duration;

use tak_core::{TakActionRecord, TakGameState};

use crate::{
    domain::{GameId, ListenerId, PlayerId},
    workflow::{gameplay::GameView, matchmaking::SeekView},
};

pub trait ListenerNotificationPort {
    fn notify_listener(&self, listener: ListenerId, message: ListenerMessage);
    fn notify_listeners(&self, listeners: &[ListenerId], message: ListenerMessage);
    fn notify_all(&self, message: ListenerMessage);
}

#[derive(Clone, Debug)]
pub enum ListenerMessage {
    SeekCreated {
        seek: SeekView,
    },
    SeekCanceled {
        seek: SeekView,
    },
    GameStarted {
        game: GameView,
    },
    GameEnded {
        game: GameView,
    },
    PlayersOnline {
        players: Vec<PlayerId>,
    },
    GameOver {
        game_id: GameId,
        game_state: TakGameState,
    },
    GameAction {
        game_id: GameId,
        player_id: PlayerId,
        action: TakActionRecord,
    },
    GameActionUndone {
        game_id: GameId,
    },
    GameTimeUpdate {
        game_id: GameId,
        white_time: Duration,
        black_time: Duration,
    },
    GameDrawOffered {
        game_id: GameId,
        offering_player_id: PlayerId,
    },
    GameDrawOfferRetracted {
        game_id: GameId,
        retracting_player_id: PlayerId,
    },
    GameUndoRequested {
        game_id: GameId,
        requesting_player_id: PlayerId,
    },
    GameUndoRequestRetracted {
        game_id: GameId,
        retracting_player_id: PlayerId,
    },
    GameRematchRequested {
        game_id: GameId,
    },
    GameRematchRequestRetracted {
        game_id: GameId,
    },
    ChatMessage {
        from_player_id: PlayerId,
        message: String,
        source: ChatMessageSource,
    },
    ServerAlert {
        message: ServerAlertMessage,
    },
}

#[derive(Clone, Debug)]
pub enum ServerAlertMessage {
    Shutdown,
    Custom(String),
}

#[derive(Clone, Debug)]
pub enum ChatMessageSource {
    Private,
    Global,
    Room(String),
}
