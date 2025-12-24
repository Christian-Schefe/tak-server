use std::sync::Arc;

use crate::app::domain::{GameId, ListenerId, game::GameService, spectator::SpectatorService};

pub trait ObserveGameUseCase {
    fn observe_game(
        &self,
        game_id: GameId,
        listener_id: ListenerId,
    ) -> Result<(), ObserveGameError>;
    fn unobserve_game(&self, game_id: GameId, listener_id: ListenerId);
}

pub enum ObserveGameError {
    GameNotFound,
}

pub struct ObserveGameUseCaseImpl<G: GameService, S: SpectatorService> {
    game_service: Arc<G>,
    spectator_service: Arc<S>,
}

impl<G: GameService, S: SpectatorService> ObserveGameUseCaseImpl<G, S> {
    pub fn new(game_service: Arc<G>, spectator_service: Arc<S>) -> Self {
        Self {
            game_service,
            spectator_service,
        }
    }
}

impl<G: GameService, S: SpectatorService> ObserveGameUseCase for ObserveGameUseCaseImpl<G, S> {
    fn observe_game(
        &self,
        game_id: GameId,
        listener_id: ListenerId,
    ) -> Result<(), ObserveGameError> {
        if self.game_service.get_game_by_id(game_id).is_none() {
            return Err(ObserveGameError::GameNotFound);
        }
        self.spectator_service.observe_game(game_id, listener_id);
        Ok(())
    }

    fn unobserve_game(&self, game_id: GameId, listener_id: ListenerId) {
        self.spectator_service.unobserve_game(game_id, listener_id);
    }
}
