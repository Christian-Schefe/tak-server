use tak_core::TakActionRecord;

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

#[derive(Clone, Debug)]
pub enum ChatMessageSource {
    Private,
    Global,
    Room(String),
}
