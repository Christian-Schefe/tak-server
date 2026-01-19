use std::sync::Arc;

use crate::{
    domain::{
        GameId, PlayerId,
        game::{GameMetadata, GameService},
        spectator::SpectatorService,
    },
    ports::{
        connection::AccountConnectionPort,
        notification::{ListenerMessage, ListenerNotificationPort},
    },
    services::player_resolver::PlayerResolverService,
};

#[async_trait::async_trait]
pub trait NotifyPlayerWorkflow {
    async fn notify_players_and_observers_of_game(
        &self,
        game: &GameMetadata,
        message: &ListenerMessage,
    );
    async fn notify_players_and_observers(&self, game_id: GameId, message: &ListenerMessage);
    async fn notify_players(&self, players: &[PlayerId], message: &ListenerMessage);
}

pub struct NotifyPlayerWorkflowImpl<
    L: ListenerNotificationPort,
    P: AccountConnectionPort,
    G: GameService,
    S: SpectatorService,
    R: PlayerResolverService,
> {
    listener_notification_port: Arc<L>,
    player_connection_port: Arc<P>,
    game_service: Arc<G>,
    spectator_service: Arc<S>,
    player_resolver_service: Arc<R>,
}

impl<
    L: ListenerNotificationPort,
    P: AccountConnectionPort,
    G: GameService,
    S: SpectatorService,
    R: PlayerResolverService,
> NotifyPlayerWorkflowImpl<L, P, G, S, R>
{
    pub fn new(
        listener_notification_port: Arc<L>,
        player_connection_port: Arc<P>,
        game_service: Arc<G>,
        spectator_service: Arc<S>,
        player_resolver_service: Arc<R>,
    ) -> Self {
        Self {
            listener_notification_port,
            player_connection_port,
            game_service,
            spectator_service,
            player_resolver_service,
        }
    }
}

#[async_trait::async_trait]
impl<
    L: ListenerNotificationPort + Send + Sync,
    P: AccountConnectionPort + Send + Sync,
    G: GameService + Send + Sync,
    S: SpectatorService + Send + Sync,
    R: PlayerResolverService + Send + Sync,
> NotifyPlayerWorkflow for NotifyPlayerWorkflowImpl<L, P, G, S, R>
{
    async fn notify_players_and_observers_of_game(
        &self,
        game: &GameMetadata,
        message: &ListenerMessage,
    ) {
        self.notify_players(&[game.white_id, game.black_id], message)
            .await;
        let observers = self.spectator_service.get_spectators_for_game(game.game_id);
        self.listener_notification_port
            .notify_listeners(&observers, message);
    }

    async fn notify_players_and_observers(&self, game_id: GameId, message: &ListenerMessage) {
        let Some(game) = self.game_service.get_game_by_id(game_id) else {
            return;
        };
        self.notify_players_and_observers_of_game(&game.metadata, message)
            .await;
    }

    async fn notify_players(&self, players: &[PlayerId], message: &ListenerMessage) {
        let listener_id_futs = players.iter().map(|player_id| async move {
            let Ok(account_id) = self
                .player_resolver_service
                .resolve_account_id_by_player_id(*player_id)
                .await
            else {
                return None;
            };
            self.player_connection_port
                .get_connection_id(&account_id)
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
