use std::sync::Arc;

use more_concurrent_maps::multi::ConcurrentMultiMap;

use crate::domain::{GameId, ListenerId};

pub trait SpectatorService {
    fn observe_game(&self, game_id: GameId, listener_id: ListenerId);
    fn unobserve_game(&self, game_id: GameId, listener_id: ListenerId);
    fn unobserve_all_games(&self, listener_id: ListenerId);
    fn get_spectators_for_game(&self, game_id: GameId) -> Vec<ListenerId>;
    fn remove_game(&self, game_id: GameId);
}

pub struct SpectatorServiceImpl {
    spectator_registry: Arc<ConcurrentMultiMap<ListenerId, GameId>>,
}

impl SpectatorServiceImpl {
    pub fn new() -> Self {
        Self {
            spectator_registry: Arc::new(ConcurrentMultiMap::new()),
        }
    }
}

impl SpectatorService for SpectatorServiceImpl {
    fn observe_game(&self, game_id: GameId, listener_id: ListenerId) {
        self.spectator_registry.insert(listener_id, game_id);
    }

    fn unobserve_game(&self, game_id: GameId, listener_id: ListenerId) {
        self.spectator_registry.remove(&listener_id, &game_id);
    }

    fn unobserve_all_games(&self, listener_id: ListenerId) {
        self.spectator_registry.remove_by_left(&listener_id);
    }

    fn get_spectators_for_game(&self, game_id: GameId) -> Vec<ListenerId> {
        self.spectator_registry.get_by_right(&game_id)
    }

    fn remove_game(&self, game_id: GameId) {
        self.spectator_registry.remove_by_right(&game_id);
    }
}
