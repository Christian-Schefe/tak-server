use std::sync::Arc;

use crate::{
    domain::{
        MatchId,
        game::GameService,
        game_history::{GameHistoryService, GameRepository},
        r#match::MatchService,
    },
    ports::notification::{ListenerMessage, ListenerNotificationPort},
    processes::game_timeout_runner::GameTimeoutRunner,
    workflow::{account::get_snapshot::GetSnapshotWorkflow, gameplay::GameView},
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
    M: MatchService,
    GH: GameHistoryService,
    GR: GameRepository,
    G: GameService,
    GT: GameTimeoutRunner,
    L: ListenerNotificationPort,
    S: GetSnapshotWorkflow,
> {
    match_service: Arc<M>,
    game_history_service: Arc<GH>,
    game_repository: Arc<GR>,
    game_service: Arc<G>,
    game_timeout_runner: Arc<GT>,
    listener_notification_port: Arc<L>,
    get_snapshot_workflow: Arc<S>,
}
impl<
    M: MatchService,
    GH: GameHistoryService,
    GR: GameRepository,
    G: GameService,
    GT: GameTimeoutRunner,
    L: ListenerNotificationPort,
    S: GetSnapshotWorkflow,
> CreateGameFromMatchWorkflowImpl<M, GH, GR, G, GT, L, S>
{
    pub fn new(
        match_service: Arc<M>,
        game_history_service: Arc<GH>,
        game_repository: Arc<GR>,
        game_service: Arc<G>,
        game_timeout_runner: Arc<GT>,
        listener_notification_port: Arc<L>,
        get_snapshot_workflow: Arc<S>,
    ) -> Self {
        Self {
            match_service,
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
    M: MatchService + Send + Sync,
    GH: GameHistoryService + Send + Sync,
    GR: GameRepository + Send + Sync,
    G: GameService + Send + Sync,
    GT: GameTimeoutRunner + Send + Sync,
    L: ListenerNotificationPort + Send + Sync,
    S: GetSnapshotWorkflow + Send + Sync,
> CreateGameFromMatchWorkflow for CreateGameFromMatchWorkflowImpl<M, GH, GR, G, GT, L, S>
{
    async fn create_game_from_match(
        &self,
        match_id: MatchId,
    ) -> Result<(), CreateGameFromMatchError> {
        let date = chrono::Utc::now();

        let Some(match_entry) = self.match_service.reserve_match_in_progress(match_id) else {
            return Err(CreateGameFromMatchError::AlreadyInProgress);
        };
        let (white_id, black_id) = match_entry.get_next_matchup_colors();

        let snapshot_white = self
            .get_snapshot_workflow
            .get_snapshot(white_id, date)
            .await;
        let snapshot_black = self
            .get_snapshot_workflow
            .get_snapshot(black_id, date)
            .await;

        let game_record = self.game_history_service.get_ongoing_game_record(
            date,
            snapshot_white,
            snapshot_black,
            match_entry.game_settings.clone(),
            match_entry.game_type,
        );

        let game_id = match self.game_repository.save_ongoing_game(game_record).await {
            Ok(id) => id,
            Err(e) => {
                log::error!(
                    "Failed to save ongoing game for match {}: {}",
                    match_entry.id,
                    e
                );
                return Err(CreateGameFromMatchError::RepositoryError);
            }
        };

        if !self
            .match_service
            .start_game_in_match(match_entry.id, game_id)
        {
            log::error!(
                "Failed to start game {} in match {}",
                game_id,
                match_entry.id
            );
            return Err(CreateGameFromMatchError::MatchNotFound);
        }

        let game = self.game_service.create_game(
            game_id,
            date,
            white_id,
            black_id,
            match_entry.game_type,
            match_entry.game_settings.clone(),
        );

        GameTimeoutRunner::schedule_game_timeout_check(
            self.game_timeout_runner.clone(),
            game.game_id,
        );

        let msg = ListenerMessage::GameStarted {
            game: GameView::from(&game),
        };
        self.listener_notification_port.notify_all(msg);
        Ok(())
    }
}
