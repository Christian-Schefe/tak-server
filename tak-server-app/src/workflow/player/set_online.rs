use std::sync::Arc;

use crate::{
    domain::{PlayerId, player::PlayerService},
    ports::notification::{ListenerMessage, ListenerNotificationPort},
};

pub trait SetPlayerOnlineUseCase {
    fn set_online(&self, player_id: PlayerId);
    fn set_offline(&self, player_id: PlayerId);
}

pub struct SetPlayerOnlineUseCaseImpl<P: PlayerService, L: ListenerNotificationPort> {
    player_service: Arc<P>,
    notification_port: Arc<L>,
}

impl<P: PlayerService, L: ListenerNotificationPort> SetPlayerOnlineUseCaseImpl<P, L> {
    pub fn new(player_service: Arc<P>, notification_port: Arc<L>) -> Self {
        Self {
            player_service,
            notification_port,
        }
    }
}

impl<P: PlayerService, L: ListenerNotificationPort> SetPlayerOnlineUseCase
    for SetPlayerOnlineUseCaseImpl<P, L>
{
    fn set_online(&self, player_id: PlayerId) {
        if let Some(players) = self.player_service.set_player_online(player_id) {
            let message = ListenerMessage::PlayersOnline { players };
            self.notification_port.notify_all(message);
        }
    }

    fn set_offline(&self, player_id: PlayerId) {
        if let Some(players) = self.player_service.set_player_offline(player_id) {
            let message = ListenerMessage::PlayersOnline { players };
            self.notification_port.notify_all(message);
        }
    }
}
