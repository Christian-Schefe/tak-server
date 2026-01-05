use std::sync::Arc;

use crate::{
    domain::{
        GameId, PlayerId,
        game::{GameMetadata, GameService},
        spectator::SpectatorService,
    },
    ports::{
        connection::PlayerConnectionPort,
        notification::{ListenerMessage, ListenerNotificationPort},
    },
};

#[async_trait::async_trait]
pub trait NotifyPlayerWorkflow {
    async fn notify_players_and_observers_of_game(
        &self,
        game: &GameMetadata,
        message: ListenerMessage,
    );
    async fn notify_players_and_observers(&self, game_id: GameId, message: ListenerMessage);
    async fn notify_players(&self, players: &[PlayerId], message: ListenerMessage);
}

pub struct NotifyPlayerWorkflowImpl<
    L: ListenerNotificationPort,
    P: PlayerConnectionPort,
    G: GameService,
    S: SpectatorService,
> {
    listener_notification_port: Arc<L>,
    player_connection_port: Arc<P>,
    game_service: Arc<G>,
    spectator_service: Arc<S>,
}

impl<L: ListenerNotificationPort, P: PlayerConnectionPort, G: GameService, S: SpectatorService>
    NotifyPlayerWorkflowImpl<L, P, G, S>
{
    pub fn new(
        listener_notification_port: Arc<L>,
        player_connection_port: Arc<P>,
        game_service: Arc<G>,
        spectator_service: Arc<S>,
    ) -> Self {
        Self {
            listener_notification_port,
            player_connection_port,
            game_service,
            spectator_service,
        }
    }
}

#[async_trait::async_trait]
impl<
    L: ListenerNotificationPort + Send + Sync,
    P: PlayerConnectionPort + Send + Sync,
    G: GameService + Send + Sync,
    S: SpectatorService + Send + Sync,
> NotifyPlayerWorkflow for NotifyPlayerWorkflowImpl<L, P, G, S>
{
    async fn notify_players_and_observers_of_game(
        &self,
        game: &GameMetadata,
        message: ListenerMessage,
    ) {
        self.notify_players(&[game.white_id, game.black_id], message.clone())
            .await;
        let observers = self.spectator_service.get_spectators_for_game(game.game_id);
        self.listener_notification_port
            .notify_listeners(&observers, message);
    }

    async fn notify_players_and_observers(&self, game_id: GameId, message: ListenerMessage) {
        let Some(game) = self.game_service.get_game_by_id(game_id) else {
            return;
        };
        self.notify_players_and_observers_of_game(&game.metadata, message)
            .await;
    }

    async fn notify_players(&self, players: &[PlayerId], message: ListenerMessage) {
        let listener_id_futs = players
            .iter()
            .map(|player_id| self.player_connection_port.get_connection_id(*player_id));
        let listener_ids = futures::future::join_all(listener_id_futs).await;
        for listener_id_opt in listener_ids {
            if let Some(connection_id) = listener_id_opt {
                self.listener_notification_port
                    .notify_listener(connection_id, message.clone());
            }
        }
    }
}
