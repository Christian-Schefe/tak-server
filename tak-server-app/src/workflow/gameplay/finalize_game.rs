use std::sync::Arc;

use crate::{
    domain::{
        game::Game,
        game_history::{GameHistoryService, GameRepository},
        r#match::MatchService,
        rating::{RatingRepository, RatingService},
    },
    workflow::account::get_snapshot::GetSnapshotWorkflow,
};

#[async_trait::async_trait]
pub trait FinalizeGameWorkflow {
    async fn finalize_game(&self, ended_game: Game);
}

pub struct FinalizeGameWorkflowImpl<
    G: GameRepository,
    R: RatingService,
    RP: RatingRepository,
    GH: GameHistoryService,
    M: MatchService,
    S: GetSnapshotWorkflow,
> {
    game_repository: Arc<G>,
    rating_service: Arc<R>,
    rating_repository: Arc<RP>,
    game_history_service: Arc<GH>,
    match_service: Arc<M>,
    get_snapshot_workflow: Arc<S>,
}

impl<
    G: GameRepository,
    R: RatingService,
    RP: RatingRepository,
    GH: GameHistoryService,
    M: MatchService,
    S: GetSnapshotWorkflow,
> FinalizeGameWorkflowImpl<G, R, RP, GH, M, S>
{
    pub fn new(
        game_repository: Arc<G>,
        rating_service: Arc<R>,
        rating_repository: Arc<RP>,
        game_history_service: Arc<GH>,
        match_service: Arc<M>,
        get_snapshot_workflow: Arc<S>,
    ) -> Self {
        Self {
            game_repository,
            rating_service,
            rating_repository,
            game_history_service,
            match_service,
            get_snapshot_workflow,
        }
    }
}

#[async_trait::async_trait]
impl<
    G: GameRepository + Send + Sync + 'static,
    R: RatingService + Send + Sync + 'static,
    RP: RatingRepository + Send + Sync + 'static,
    GH: GameHistoryService + Send + Sync + 'static,
    M: MatchService + Send + Sync + 'static,
    S: GetSnapshotWorkflow + Send + Sync + 'static,
> FinalizeGameWorkflow for FinalizeGameWorkflowImpl<G, R, RP, GH, M, S>
{
    async fn finalize_game(&self, ended_game: Game) {
        let Some(finished_game_id) = self
            .game_history_service
            .remove_ongoing_game_id(ended_game.game_id)
        else {
            return;
        };

        let snapshot_white = self
            .get_snapshot_workflow
            .get_snapshot(ended_game.white, ended_game.date)
            .await;
        let snapshot_black = self
            .get_snapshot_workflow
            .get_snapshot(ended_game.black, ended_game.date)
            .await;

        let game_rating_info = self
            .rating_repository
            .update_player_ratings(ended_game.white, ended_game.black, |w_rating, b_rating| {
                self.rating_service.calculate_ratings(
                    ended_game.date,
                    &ended_game,
                    w_rating,
                    b_rating,
                )
            })
            .await;

        let match_id = ended_game.match_id;
        let game_record_update = self.game_history_service.get_finished_game_record_update(
            ended_game,
            snapshot_white,
            snapshot_black,
            game_rating_info,
        );

        self.match_service
            .end_game_in_match(match_id, finished_game_id);

        if let Err(_) = self
            .game_repository
            .update_finished_game(finished_game_id, game_record_update)
            .await
        {
            //TODO: log error
        }
    }
}
