use std::sync::Arc;

use tak_core::TakPlayer;

use crate::{
    PortsCollection,
    ports::{
        game_history::GameRepository,
        matches::MatchRepository,
        notification::{ListenerMessage, ListenerNotificationPort},
        transaction::{TransactionError, TransactionManager},
    },
    processes::game_timeout_runner::GameTimeoutRunner,
    workflow::{account::get_snapshot::GetSnapshotWorkflow, gameplay::OngoingGameView},
};
use tak_domain::{MatchId, game::GameService, game_history::GameRecord};

#[async_trait::async_trait]
pub trait CreateGameFromMatchWorkflow {
    async fn create_game_from_match(
        &self,
        match_id: MatchId,
    ) -> Result<(), CreateGameFromMatchError>;
}

#[derive(Debug)]
pub enum CreateGameFromMatchError {
    MatchNotFound,
    Internal,
    AlreadyInProgress,
}

pub struct CreateGameFromMatchWorkflowImpl<
    Ports: PortsCollection,
    G: GameService,
    GT: GameTimeoutRunner,
    L: ListenerNotificationPort,
    S: GetSnapshotWorkflow,
> {
    ports: Arc<Ports>,
    game_service: Arc<G>,
    game_timeout_runner: Arc<GT>,
    listener_notification_port: Arc<L>,
    get_snapshot_workflow: Arc<S>,
}
impl<
    Ports: PortsCollection,
    G: GameService,
    GT: GameTimeoutRunner,
    L: ListenerNotificationPort,
    S: GetSnapshotWorkflow,
> CreateGameFromMatchWorkflowImpl<Ports, G, GT, L, S>
{
    pub fn new(
        ports: Arc<Ports>,
        game_service: Arc<G>,
        game_timeout_runner: Arc<GT>,
        listener_notification_port: Arc<L>,
        get_snapshot_workflow: Arc<S>,
    ) -> Self {
        Self {
            ports,
            game_service,
            game_timeout_runner,
            listener_notification_port,
            get_snapshot_workflow,
        }
    }
}

#[async_trait::async_trait]
impl<
    Ports: PortsCollection + Send + Sync + 'static,
    G: GameService + Send + Sync,
    GT: GameTimeoutRunner + Send + Sync,
    L: ListenerNotificationPort + Send + Sync,
    S: GetSnapshotWorkflow + Send + Sync,
> CreateGameFromMatchWorkflow for CreateGameFromMatchWorkflowImpl<Ports, G, GT, L, S>
{
    #[tracing::instrument(skip(self))]
    async fn create_game_from_match(
        &self,
        match_id: MatchId,
    ) -> Result<(), CreateGameFromMatchError> {
        let date = chrono::Utc::now();

        let mut match_entry = match self
            .ports
            .transaction_manager()
            .execute(self.ports.match_repository().get_match(match_id))
            .await
        {
            Ok(Some(match_entry)) => match_entry,
            Ok(None) => {
                tracing::error!("Match {} not found", match_id);
                return Err(CreateGameFromMatchError::MatchNotFound);
            }
            Err(TransactionError(e)) => {
                tracing::error!("Error while retrieving match {}: {}", match_id, e);
                return Err(CreateGameFromMatchError::Internal);
            }
        };
        let player1_color = match_entry.try_begin_game().map_err(|e| {
            tracing::error!("Failed to start game for match {}: {}", match_id, e);
            CreateGameFromMatchError::AlreadyInProgress
        })?;

        let (white_id, black_id) = match player1_color {
            TakPlayer::White => (match_entry.player1.user_id, match_entry.player2.user_id),
            TakPlayer::Black => (match_entry.player2.user_id, match_entry.player1.user_id),
        };

        let snapshot_white = match self
            .get_snapshot_workflow
            .get_snapshot(white_id, date)
            .await
        {
            Ok(snapshot) => snapshot,
            Err(_) => return Err(CreateGameFromMatchError::Internal),
        };
        let snapshot_black = match self
            .get_snapshot_workflow
            .get_snapshot(black_id, date)
            .await
        {
            Ok(snapshot) => snapshot,
            Err(_) => return Err(CreateGameFromMatchError::Internal),
        };

        let metadata = self.game_service.create_game_metadata(
            date,
            white_id,
            black_id,
            match_entry.settings.is_rated,
            match_entry.settings.game_settings.clone(),
            Some(match_id),
        );

        let game_record =
            GameRecord::for_ongoing_game(metadata.clone(), snapshot_white, snapshot_black);

        let game_id = match self
            .ports
            .transaction_manager()
            .execute(
                self.ports.game_repository().save_ongoing_game(game_record),
            )
            .await
        {
            Ok(id) => id,
            Err(TransactionError(e)) => {
                tracing::error!("Failed to save ongoing game for match {}: {}", match_id, e);
                return Err(CreateGameFromMatchError::Internal);
            }
        };

        if let Err(TransactionError(e)) = self
            .ports
            .transaction_manager()
            .execute(
                self.ports
                    .match_repository()
                    .update_match(match_id, match_entry),
            )
            .await
        {
            tracing::error!(
                "Failed to start game {} in match {}: {}",
                game_id,
                match_id,
                e
            );
            return Err(CreateGameFromMatchError::MatchNotFound);
        }
        let game = self.game_service.create_game(game_id, metadata);
        GameTimeoutRunner::schedule_game_timeout_check(self.game_timeout_runner.clone(), game_id);
        let msg = ListenerMessage::GameStarted {
            game: OngoingGameView::from(&game),
        };
        self.listener_notification_port.notify_all(&msg);

        Ok(())
    }
}
