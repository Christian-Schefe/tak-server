use std::sync::Arc;

use crate::{
    domain::{
        game::GameService,
        game_history::{GameHistoryService, GameRepository},
        r#match::{Match, MatchService},
    },
    processes::game_timeout_runner::GameTimeoutRunner,
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
> {
    match_service: Arc<M>,
    game_history_service: Arc<GH>,
    game_repository: Arc<GR>,
    game_service: Arc<G>,
    game_timeout_runner: Arc<GT>,
}
impl<
    M: MatchService,
    GH: GameHistoryService,
    GR: GameRepository,
    G: GameService,
    GT: GameTimeoutRunner,
> CreateGameFromMatchWorkflowImpl<M, GH, GR, G, GT>
{
    pub fn new(
        match_service: Arc<M>,
        game_history_service: Arc<GH>,
        game_repository: Arc<GR>,
        game_service: Arc<G>,
        game_timeout_runner: Arc<GT>,
    ) -> Self {
        Self {
            match_service,
            game_history_service,
            game_repository,
            game_service,
            game_timeout_runner,
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
> CreateGameFromMatchWorkflow for CreateGameFromMatchWorkflowImpl<M, GH, GR, G, GT>
{
    async fn create_game_from_match(&self, match_entry: &Match) {
        let date = chrono::Utc::now();
        let game = self.game_service.create_game(
            date,
            match_entry.player1,
            match_entry.player2,
            match_entry.inital_color,
            match_entry.game_type,
            match_entry.game_settings.clone(),
            match_entry.id,
        );

        let game_record = self.game_history_service.get_ongoing_game_record(
            date,
            game.white,
            game.black,
            game.settings.clone(),
            game.game_type,
        );

        let finished_game_id = match self.game_repository.save_ongoing_game(game_record).await {
            Ok(id) => id,
            Err(_) => {
                // TODO: log error
                return;
            }
        };

        self.game_history_service
            .save_ongoing_game_id(game.game_id, finished_game_id);

        self.match_service
            .start_game_in_match(match_entry.id, game.game_id);

        GameTimeoutRunner::schedule_game_timeout_check(
            self.game_timeout_runner.clone(),
            game.game_id,
        );
    }
}
