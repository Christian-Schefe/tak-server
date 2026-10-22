use tak_core::{TakAction, TakGameResult, TakTimeInfo};

use crate::workflow::{chat::ChatMessageView, matchmaking::SeekView};
use tak_domain::{
    GameId, ListenerId, MatchId, UserId, chat::ChatConversation, game::request::GameRequest,
    game_record::GameRecord,
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
    SeekCancelled {
        seek: SeekView,
    },
    SeekAccepted {
        seek: SeekView,
    },
    GameStarted {
        game_id: GameId,
        game: GameRecord,
    },
    GameEnded {
        game_id: GameId,
        game: GameRecord,
    },
    UsersOnline {
        users: Vec<UserId>,
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
    MatchReadinessChanged { user_id: Option<UserId> },
}

#[derive(Clone, Debug)]
pub enum ListenerGameMessageType {
    GameOver {
        game_result: TakGameResult,
    },
    GameAction {
        user_id: UserId,
        action: TakAction,
        ply_index: usize,
    },
    GameActionUndone {
        ply_index: usize,
    },
    GameRequestChanged {
        user_id: UserId,
        request: GameRequest,
    },
}

#[derive(Clone, Debug)]
pub enum ServerAlertMessage {
    Shutdown,
    Custom(String),
}
