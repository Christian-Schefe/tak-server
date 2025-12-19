use std::sync::Arc;

use crate::{
    app::{
        event::EventListener,
        ports::notification::{ListenerMessage, ListenerNotificationPort},
    },
    domain::player::PlayerEvent,
};

pub struct PlayerEventNotifier<L: ListenerNotificationPort> {
    notification_port: Arc<L>,
}

impl<L: ListenerNotificationPort> PlayerEventNotifier<L> {
    pub fn new(notification_port: Arc<L>) -> Self {
        Self { notification_port }
    }
}

impl<L: ListenerNotificationPort> EventListener<PlayerEvent> for PlayerEventNotifier<L> {
    fn on_event(&self, player_event: &PlayerEvent) {
        let message = match player_event {
            PlayerEvent::PlayersOnline(players) => ListenerMessage::PlayersOnline {
                players: players.clone(),
            },
        };
        self.notification_port.notify_all(message);
    }
}
