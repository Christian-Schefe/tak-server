use std::sync::Arc;

use crate::{
    app::event::EventListener,
    domain::{
        game::GameEvent,
        game_history::{GameHistoryService, GameRepository},
        r#match::MatchService,
        rating::{RatingRepository, RatingService},
    },
};

pub struct FinalizeGameListener<
    G: GameRepository,
    R: RatingService,
    RP: RatingRepository,
    GH: GameHistoryService,
    M: MatchService,
> {
    game_repository: Arc<G>,
    rating_service: Arc<R>,
    rating_repository: Arc<RP>,
    game_history_service: Arc<GH>,
    match_service: Arc<M>,
}

impl<
    G: GameRepository,
    R: RatingService,
    RP: RatingRepository,
    GH: GameHistoryService,
    M: MatchService,
> FinalizeGameListener<G, R, RP, GH, M>
{
    pub fn new(
        game_repository: Arc<G>,
        rating_service: Arc<R>,
        rating_repository: Arc<RP>,
        game_history_service: Arc<GH>,
        match_service: Arc<M>,
    ) -> Self {
        Self {
            game_repository,
            rating_service,
            rating_repository,
            game_history_service,
            match_service,
        }
    }
}

impl<
    G: GameRepository,
    R: RatingService,
    RP: RatingRepository,
    GH: GameHistoryService,
    M: MatchService,
> EventListener<GameEvent> for FinalizeGameListener<G, R, RP, GH, M>
{
    fn on_event(&self, event: &GameEvent) {
        if let GameEvent::Ended(game_id, ended_game) = event {
            let Some(finished_game_id) = self.game_history_service.remove_ongoing_game_id(*game_id)
            else {
                return;
            };
            let now = chrono::Utc::now();
            let white_rating = self.rating_repository.get_player_rating(ended_game.white);
            let black_rating = self.rating_repository.get_player_rating(ended_game.black);
            let game_rating_info = if let Some(rating_result) = self
                .rating_service
                .calculate_ratings(now, &ended_game, white_rating, black_rating)
            {
                self.rating_repository
                    .save_player_rating(ended_game.white, &rating_result.white_rating);
                self.rating_repository
                    .save_player_rating(ended_game.black, &rating_result.black_rating);
                Some(rating_result.game_rating_info)
            } else {
                None
            };
            let game_record = self
                .game_history_service
                .get_finished_game_record(ended_game.clone(), game_rating_info.clone());

            self.game_repository
                .update_finished_game(finished_game_id, game_record);

            self.match_service
                .end_game_in_match(ended_game.match_id, finished_game_id);
        }
    }
}
