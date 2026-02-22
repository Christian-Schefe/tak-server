use tak_core::{TakAction, TakGameResult, TakTimeInfo};

use crate::{
    domain::{AccountId, GameId, ListenerId, PlayerId, game::request::GameRequest},
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
    SeekAccepted {
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
    GameEvent {
        game_id: GameId,
        event_type: ListenerGameMessageType,
        time_info: TakTimeInfo,
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
pub enum ListenerGameMessageType {
    GameOver {
        game_result: TakGameResult,
    },
    GameAction {
        player_id: PlayerId,
        action: TakAction,
        ply_index: usize,
    },
    GameActionUndone {
        ply_index: usize,
    },
    GameRequestAdded {
        requesting_player_id: PlayerId,
        request: GameRequest,
    },
    GameRequestRetracted {
        retracting_player_id: PlayerId,
        request: GameRequest,
    },
    GameRequestRejected {
        rejecting_player_id: PlayerId,
        request: GameRequest,
    },
    GameRequestAccepted {
        accepting_player_id: PlayerId,
        request: GameRequest,
    },
}

#[derive(Clone, Debug)]
pub enum ServerAlertMessage {
    Shutdown,
    Custom(String),
}
