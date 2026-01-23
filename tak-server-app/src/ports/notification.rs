use std::time::Duration;

use tak_core::{TakGameResult, TakRequest};

use crate::{
    domain::{AccountId, GameId, ListenerId, PlayerId, game::GameActionRecord},
    workflow::{
        chat::message::MessageTarget,
        gameplay::{FinishedGameView, OngoingGameView},
        matchmaking::SeekView,
    },
};

pub trait ListenerNotificationPort {
    fn notify_listener(&self, listener: ListenerId, message: &ListenerMessage);
    fn notify_listeners(&self, listeners: &[ListenerId], message: &ListenerMessage) {
        for listener in listeners {
            self.notify_listener(*listener, message);
        }
    }
    fn notify_all(&self, message: &ListenerMessage);
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
        game: OngoingGameView,
    },
    GameEnded {
        game: FinishedGameView,
    },
    AccountsOnline {
        accounts: Vec<AccountId>,
    },
    GameOver {
        game_id: GameId,
        game_result: TakGameResult,
    },
    GameAction {
        game_id: GameId,
        player_id: PlayerId,
        action: GameActionRecord,
    },
    GameActionUndone {
        game_id: GameId,
    },
    GameTimeUpdate {
        game_id: GameId,
        white_time: Duration,
        black_time: Duration,
    },
    GameRequestAdded {
        game_id: GameId,
        requesting_player_id: PlayerId,
        request: TakRequest,
    },
    GameRequestRetracted {
        game_id: GameId,
        retracting_player_id: PlayerId,
        request: TakRequest,
    },
    GameRequestRejected {
        game_id: GameId,
        rejecting_player_id: PlayerId,
        request: TakRequest,
    },
    GameRequestAccepted {
        game_id: GameId,
        accepting_player_id: PlayerId,
        request: TakRequest,
    },

    GameRematchRequested {
        game_id: GameId,
    },
    GameRematchRequestRetracted {
        game_id: GameId,
    },
    ChatMessage {
        from_account_id: AccountId,
        message: String,
        target: MessageTarget,
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
