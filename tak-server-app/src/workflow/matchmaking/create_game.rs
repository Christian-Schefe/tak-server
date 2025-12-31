use std::sync::Arc;

use crate::{
    domain::{
        game::GameService,
        game_history::{GameHistoryService, GameRepository},
        r#match::{Match, MatchService},
    },
    ports::notification::{ListenerMessage, ListenerNotificationPort},
    processes::game_timeout_runner::GameTimeoutRunner,
    workflow::gameplay::GameView,
};

#[async_trait::async_trait]
pub trait CreateGameFromMatchWorkflow {
    async fn create_game_from_match(&self, match_entry: &Match);
}

pub struct CreateGameFromMatchWorkflowImpl<
    M: MatchService,
    GH: GameHistoryService,
    GR: GameRepository,
    G: GameService,
    GT: GameTimeoutRunner,
    L: ListenerNotificationPort,
> {
    match_service: Arc<M>,
    game_history_service: Arc<GH>,
    game_repository: Arc<GR>,
    game_service: Arc<G>,
    game_timeout_runner: Arc<GT>,
    listener_notification_port: Arc<L>,
}
impl<
    M: MatchService,
    GH: GameHistoryService,
    GR: GameRepository,
    G: GameService,
    GT: GameTimeoutRunner,
    L: ListenerNotificationPort,
> CreateGameFromMatchWorkflowImpl<M, GH, GR, G, GT, L>
{
    pub fn new(
        match_service: Arc<M>,
        game_history_service: Arc<GH>,
        game_repository: Arc<GR>,
        game_service: Arc<G>,
        game_timeout_runner: Arc<GT>,
        listener_notification_port: Arc<L>,
    ) -> Self {
        Self {
            match_service,
            game_history_service,
            game_repository,
            game_service,
            game_timeout_runner,
            listener_notification_port,
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
> CreateGameFromMatchWorkflow for CreateGameFromMatchWorkflowImpl<M, GH, GR, G, GT, L>
{
    async fn create_game_from_match(&self, match_entry: &Match) {
        let date = chrono::Utc::now();

        let (white_id, black_id) = match_entry.get_next_matchup_colors();

        let game_record = self.game_history_service.get_ongoing_game_record(
            date,
            white_id,
            black_id,
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
                return;
            }
        };

        let game = self.game_service.create_game(
            game_id,
            date,
            white_id,
            black_id,
            match_entry.game_type,
            match_entry.game_settings.clone(),
            match_entry.id,
        );

        self.match_service
            .start_game_in_match(match_entry.id, game.game_id);

        GameTimeoutRunner::schedule_game_timeout_check(
            self.game_timeout_runner.clone(),
            game.game_id,
        );

        let msg = ListenerMessage::GameStarted {
            game: GameView::from(&game),
        };
        self.listener_notification_port.notify_all(msg);
    }
}
