use std::sync::Arc;

use crate::ports::{
    connection::UserConnectionPort,
    notification::{ListenerMessage, ListenerNotificationPort},
};
use tak_domain::{GameId, UserId, game::GameMetadata, spectator::SpectatorService};

#[async_trait::async_trait]
pub trait NotifyPlayerWorkflow {
    async fn notify_players_and_observers_of_game(
        &self,
        game_id: GameId,
        metadata: &GameMetadata,
        message: &ListenerMessage,
    );
    async fn notify_players(&self, players: &[UserId], message: &ListenerMessage);
}

pub struct NotifyPlayerWorkflowImpl<
    L: ListenerNotificationPort,
    P: UserConnectionPort,
    S: SpectatorService,
> {
    listener_notification_port: Arc<L>,
    player_connection_port: Arc<P>,
    spectator_service: Arc<S>,
}

impl<L: ListenerNotificationPort, P: UserConnectionPort, S: SpectatorService>
    NotifyPlayerWorkflowImpl<L, P, S>
{
    pub fn new(
        listener_notification_port: Arc<L>,
        player_connection_port: Arc<P>,
        spectator_service: Arc<S>,
    ) -> Self {
        Self {
            listener_notification_port,
            player_connection_port,
            spectator_service,
        }
    }
}

#[async_trait::async_trait]
impl<
    L: ListenerNotificationPort + Send + Sync,
    P: UserConnectionPort + Send + Sync,
    S: SpectatorService + Send + Sync,
> NotifyPlayerWorkflow for NotifyPlayerWorkflowImpl<L, P, S>
{
    async fn notify_players_and_observers_of_game(
        &self,
        game_id: GameId,
        metadata: &GameMetadata,
        message: &ListenerMessage,
    ) {
        self.notify_players(&[metadata.white.user_id, metadata.black.user_id], message)
            .await;
        let observers = self.spectator_service.get_spectators_for_game(game_id);
        self.listener_notification_port
            .notify_listeners(&observers, message);
    }

    async fn notify_players(&self, players: &[UserId], message: &ListenerMessage) {
        let listener_id_futs = players.iter().map(|user_id| async move {
            self.player_connection_port
                .get_connection_id(*user_id)
                .await
        });
        let listener_ids = futures::future::join_all(listener_id_futs).await;
        for listener_id_opt in listener_ids {
            if let Some(connection_id) = listener_id_opt {
                self.listener_notification_port
                    .notify_listener(connection_id, message);
            }
        }
    }
}
