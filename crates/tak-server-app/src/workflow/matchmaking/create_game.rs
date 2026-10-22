use std::sync::Arc;

use tak_core::TakPlayer;

use crate::{
    PortsCollection,
    ports::{
        game::{GameRepository, OngoingGameRepository},
        matches::MatchRepository,
        notification::{ListenerMessage, ListenerNotificationPort},
        transaction::RepoError,
    },
    processes::game_timeout_runner::GameTimeoutRunner,
    workflow::account::get_snapshot::GetSnapshotWorkflow,
};
use tak_domain::{
    MatchId,
    game::{Game, GameMetadata},
    game_record::GameRecord,
};

#[async_trait::async_trait]
pub trait CreateGameFromMatchWorkflow<RpCtx> {
    async fn create_game_from_match(
        &self,
        ctx: &RpCtx,
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
    GT: GameTimeoutRunner,
    L: ListenerNotificationPort,
    S: GetSnapshotWorkflow,
> {
    ports: Arc<Ports>,
    game_timeout_runner: Arc<GT>,
    listener_notification_port: Arc<L>,
    get_snapshot_workflow: Arc<S>,
}

impl<
    Ports: PortsCollection,
    GT: GameTimeoutRunner,
    L: ListenerNotificationPort,
    S: GetSnapshotWorkflow,
> CreateGameFromMatchWorkflowImpl<Ports, GT, L, S>
{
    pub fn new(
        ports: Arc<Ports>,
        game_timeout_runner: Arc<GT>,
        listener_notification_port: Arc<L>,
        get_snapshot_workflow: Arc<S>,
    ) -> Self {
        Self {
            ports,
            game_timeout_runner,
            listener_notification_port,
            get_snapshot_workflow,
        }
    }
}

#[async_trait::async_trait]
impl<
    Ports: PortsCollection + Send + Sync + 'static,
    GT: GameTimeoutRunner + Send + Sync,
    L: ListenerNotificationPort + Send + Sync,
    S: GetSnapshotWorkflow + Send + Sync,
> CreateGameFromMatchWorkflow<Ports::TrCtx> for CreateGameFromMatchWorkflowImpl<Ports, GT, L, S>
{
    #[tracing::instrument(skip(self, ctx))]
    async fn create_game_from_match(
        &self,
        ctx: &Ports::TrCtx,
        match_id: MatchId,
    ) -> Result<(), CreateGameFromMatchError> {
        let date = chrono::Utc::now();

        let mut match_entry = match self.ports.match_repository().get_match(ctx, match_id).await {
            Ok(Some(match_entry)) => match_entry,
            Ok(None) => {
                tracing::error!("Match {} not found", match_id);
                return Err(CreateGameFromMatchError::MatchNotFound);
            }
            Err(RepoError(e)) => {
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

        let metadata = GameMetadata::new(
            date,
            snapshot_white,
            snapshot_black,
            match_entry.settings.is_rated,
            match_entry.settings.game_settings.clone(),
            Some(match_id),
        );

        let now = std::time::SystemTime::now();
        let game_state = Game::new(&metadata.settings, now);
        let game_record = GameRecord::new_ongoing(metadata);

        let game_id = match self
            .ports
            .game_repository()
            .create_game(ctx, game_record.clone())
            .await
        {
            Ok(id) => id,
            Err(RepoError(e)) => {
                tracing::error!("Failed to save ongoing game for match {}: {}", match_id, e);
                return Err(CreateGameFromMatchError::Internal);
            }
        };

        match self
            .ports
            .ongoing_game_repository()
            .set_game(&ctx, game_id, game_state)
            .await
        {
            Ok(()) => (),
            Err(RepoError(e)) => {
                tracing::error!(
                    "Failed to save ongoing game state for match {}: {}",
                    match_id,
                    e
                );
                return Err(CreateGameFromMatchError::Internal);
            }
        }

        if let Err(RepoError(e)) = self
            .ports
            .match_repository()
            .update_match(ctx, match_id, match_entry)
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
        GameTimeoutRunner::schedule_game_timeout_check(self.game_timeout_runner.clone(), game_id);
        let msg = ListenerMessage::GameStarted {
            game_id,
            game: game_record,
        };
        self.listener_notification_port.notify_all(&msg);

        Ok(())
    }
}
