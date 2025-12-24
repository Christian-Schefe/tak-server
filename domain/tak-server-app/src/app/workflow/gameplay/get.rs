use std::sync::Arc;

use crate::app::{
    domain::{GameId, game::GameService},
    workflow::gameplay::GameView,
};

pub trait GetOngoingGameUseCase {
    fn get_game(&self, game_id: GameId) -> Option<GameView>;
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
    fn get_game(&self, game_id: GameId) -> Option<GameView> {
        self.game_service
            .get_game_by_id(game_id)
            .map(|game| GameView::from(game))
    }
}
