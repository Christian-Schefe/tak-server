use tak_core::{TakAction, TakGameResult, TakPlayer, TakTimeInfo};

use crate::application::{chat::ChatMessageView, matchmaking::SeekView};
use tak_domain::{
    GameId, MatchId, UserId, chat::ChatConversation, game::GameRequest, game_record::GameRecord,
};

pub trait NotificationPort {
    fn notify(&self, message: &NotificationMessage);
}

#[derive(Clone, Debug)]
pub enum NotificationMessage {
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
        event_type: NotificationGameMessageType,
        time_info: TakTimeInfo,
    },
    MatchEvent {
        match_id: MatchId,
        event_type: NotificationMatchEventType,
    },
    ChatMessage {
        message: ChatMessageView,
        conversation: ChatConversation,
    },
}

#[derive(Clone, Debug)]
pub enum NotificationMatchEventType {
    MatchReadinessChanged { user_id: Option<UserId> },
}

#[derive(Clone, Debug)]
pub enum NotificationGameMessageType {
    GameOver {
        game_result: TakGameResult,
    },
    GameAction {
        action: TakAction,
        ply_index: usize,
    },
    GameActionUndone {
        ply_index: usize,
    },
    GameRequestChanged {
        player: TakPlayer,
        request: GameRequest,
    },
}
