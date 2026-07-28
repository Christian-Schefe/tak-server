use std::sync::Arc;

use crate::{
    domain::{GameId, game::GameService},
    workflow::gameplay::OngoingGameView,
};

pub trait GetOngoingGameUseCase {
    fn get_game(&self, game_id: GameId) -> Option<OngoingGameView>;
}

pub struct GetOngoingGameUseCaseImpl<G: GameService> {
    game_service: Arc<G>,
}

impl<G: GameService> GetOngoingGameUseCaseImpl<G> {
    pub fn new(game_service: Arc<G>) -> Self {
        Self { game_service }
    }
}

impl<G: GameService> GetOngoingGameUseCase for GetOngoingGameUseCaseImpl<G> {
    fn get_game(&self, game_id: GameId) -> Option<OngoingGameView> {
        self.game_service
            .get_game_by_id(game_id)
            .map(|game| OngoingGameView::from(game))
    }
}
