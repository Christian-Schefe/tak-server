use tak_core::{TakAction, TakGameResult, TakTimeInfo};

use crate::{
    domain::{
        AccountId, GameId, ListenerId, MatchId, PlayerId, chat::ChatConversation,
        game::request::GameRequest,
    },
    workflow::{
        chat::ChatMessageView,
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
    MatchEvent {
        match_id: MatchId,
        event_type: ListenerMatchEventType,
    },
    ChatMessage {
        message: ChatMessageView,
        conversation: ChatConversation,
    },
    ServerAlert {
        message: ServerAlertMessage,
    },
}

#[derive(Clone, Debug)]
pub enum ListenerMatchEventType {
    MatchRematchRequestAdded { requesting_player_id: PlayerId },
    MatchRematchRequestRemoved,
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
