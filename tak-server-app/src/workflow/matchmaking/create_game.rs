use std::sync::Arc;

use tak_core::TakPlayer;

use crate::{
    domain::{
        MatchId, RepoRetrieveError,
        game::GameService,
        game_history::{GameHistoryService, GameRepository},
        matches::MatchRepository,
    },
    ports::notification::{ListenerMessage, ListenerNotificationPort},
    processes::game_timeout_runner::GameTimeoutRunner,
    workflow::{account::get_snapshot::GetSnapshotWorkflow, gameplay::OngoingGameView},
};

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
    RepositoryError,
    AlreadyInProgress,
}

pub struct CreateGameFromMatchWorkflowImpl<
    M: MatchRepository,
    GH: GameHistoryService,
    GR: GameRepository,
    G: GameService,
    GT: GameTimeoutRunner,
    L: ListenerNotificationPort,
    S: GetSnapshotWorkflow,
> {
    match_repo: Arc<M>,
    game_history_service: Arc<GH>,
    game_repository: Arc<GR>,
    game_service: Arc<G>,
    game_timeout_runner: Arc<GT>,
    listener_notification_port: Arc<L>,
    get_snapshot_workflow: Arc<S>,
}
impl<
    M: MatchRepository,
    GH: GameHistoryService,
    GR: GameRepository,
    G: GameService,
    GT: GameTimeoutRunner,
    L: ListenerNotificationPort,
    S: GetSnapshotWorkflow,
> CreateGameFromMatchWorkflowImpl<M, GH, GR, G, GT, L, S>
{
    pub fn new(
        match_repo: Arc<M>,
        game_history_service: Arc<GH>,
        game_repository: Arc<GR>,
        game_service: Arc<G>,
        game_timeout_runner: Arc<GT>,
        listener_notification_port: Arc<L>,
        get_snapshot_workflow: Arc<S>,
    ) -> Self {
        Self {
            match_repo,
            game_history_service,
            game_repository,
            game_service,
            game_timeout_runner,
            listener_notification_port,
            get_snapshot_workflow,
        }
    }
}

#[async_trait::async_trait]
impl<
    M: MatchRepository + Send + Sync,
    GH: GameHistoryService + Send + Sync,
    GR: GameRepository + Send + Sync,
    G: GameService + Send + Sync,
    GT: GameTimeoutRunner + Send + Sync,
    L: ListenerNotificationPort + Send + Sync,
    S: GetSnapshotWorkflow + Send + Sync,
> CreateGameFromMatchWorkflow for CreateGameFromMatchWorkflowImpl<M, GH, GR, G, GT, L, S>
{
    #[tracing::instrument(skip(self))]
    async fn create_game_from_match(
        &self,
        match_id: MatchId,
    ) -> Result<(), CreateGameFromMatchError> {
        let date = chrono::Utc::now();

        let mut match_entry = self
            .match_repo
            .get_match(match_id)
            .await
            .map_err(|e| match e {
                RepoRetrieveError::NotFound => CreateGameFromMatchError::MatchNotFound,
                RepoRetrieveError::StorageError(e) => {
                    tracing::error!("Storage error while retrieving match {}: {}", match_id, e);
                    CreateGameFromMatchError::RepositoryError
                }
            })?;
        let player1_color = match_entry.try_begin_game().map_err(|e| {
            tracing::error!("Failed to start game for match {}: {}", match_id, e);
            CreateGameFromMatchError::AlreadyInProgress
        })?;

        let (white_id, black_id) = match player1_color {
            TakPlayer::White => (match_entry.player1, match_entry.player2),
            TakPlayer::Black => (match_entry.player2, match_entry.player1),
        };

        let snapshot_white = self
            .get_snapshot_workflow
            .get_snapshot(white_id, date)
            .await;
        let snapshot_black = self
            .get_snapshot_workflow
            .get_snapshot(black_id, date)
            .await;

        let metadata = self.game_service.create_game_metadata(
            date,
            white_id,
            black_id,
            match_entry.settings.is_rated,
            match_entry.settings.game_settings.clone(),
            Some(match_id),
        );

        let game_record = self.game_history_service.get_ongoing_game_record(
            metadata.clone(),
            snapshot_white,
            snapshot_black,
        );

        let game_id = match self.game_repository.save_ongoing_game(game_record).await {
            Ok(id) => id,
            Err(e) => {
                tracing::error!("Failed to save ongoing game for match {}: {}", match_id, e);
                return Err(CreateGameFromMatchError::RepositoryError);
            }
        };

        if let Err(e) = self.match_repo.update_match(match_id, match_entry).await {
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
