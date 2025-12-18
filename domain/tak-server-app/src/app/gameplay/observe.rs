use std::sync::Arc;

use crate::domain::{GameId, ListenerId, game::GameService};

pub trait ObserveGameUseCase {
    fn observe_game(
        &self,
        game_id: GameId,
        listener_id: ListenerId,
    ) -> Result<(), ObserveGameError>;
    fn unobserve_game(&self, game_id: GameId, listener_id: ListenerId);
    fn unobserve_all_games(&self, listener_id: ListenerId);
}

pub enum ObserveGameError {
    GameNotFound,
}

pub struct ObserveGameUseCaseImpl<G: GameService> {
    game_service: Arc<G>,
}

impl<G: GameService> ObserveGameUseCaseImpl<G> {
    pub fn new(game_service: Arc<G>) -> Self {
        Self { game_service }
    }
}

impl<G: GameService> ObserveGameUseCase for ObserveGameUseCaseImpl<G> {
    fn observe_game(
        &self,
        game_id: GameId,
        listener_id: ListenerId,
    ) -> Result<(), ObserveGameError> {
        if !self.game_service.observe_game(game_id, listener_id) {
            return Err(ObserveGameError::GameNotFound);
        }
        Ok(())
    }

    fn unobserve_game(&self, game_id: GameId, listener_id: ListenerId) {
        self.game_service.unobserve_game(game_id, listener_id);
    }

    fn unobserve_all_games(&self, listener_id: ListenerId) {
        self.game_service.unobserve_all_games(listener_id);
    }
}
