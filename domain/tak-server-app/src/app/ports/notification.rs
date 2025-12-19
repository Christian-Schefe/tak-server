use tak_core::TakActionRecord;

use crate::{
    app::matchmaking::SeekView,
    domain::{GameId, ListenerId, PlayerId},
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
}
