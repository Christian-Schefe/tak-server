use tak_core::TakActionRecord;

use crate::app::{
    domain::{GameId, ListenerId, PlayerId},
    workflow::matchmaking::SeekView,
};

pub trait ListenerNotificationPort {
    fn notify_listener(&self, listener: ListenerId, message: ListenerMessage);
    fn notify_listeners(&self, listeners: &[ListenerId], message: ListenerMessage);
    fn notify_all(&self, message: ListenerMessage);
}

pub enum ListenerMessage {
    SeekCreated {
        seek: SeekView,
    },
    SeekCanceled {
        seek: SeekView,
    },
    PlayersOnline {
        players: Vec<PlayerId>,
    },
    GameAction {
        game_id: GameId,
        action: TakActionRecord,
    },
    GameDrawOffered {
        game_id: GameId,
    },
    GameUndoRequested {
        game_id: GameId,
    },
    GameRematchRequested {
        game_id: GameId,
    },
    ChatMessage {
        from_player_id: PlayerId,
        message: String,
        source: ChatMessageSource,
    },
}

pub enum ChatMessageSource {
    Private,
    Global,
    Room(String),
}
