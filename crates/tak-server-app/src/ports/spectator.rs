use std::sync::Arc;

use more_concurrent_maps::multi::ConcurrentMultiMap;
use tak_domain::{GameId, UserId};

pub trait SpectatorService {
    fn observe_game(&self, user_id: UserId, game_id: GameId);
    fn unobserve_game(&self, user_id: UserId, game_id: GameId);
    fn unobserve_all_games(&self, user_id: UserId);
    fn get_spectators_for_game(&self, game_id: GameId) -> Vec<UserId>;
    fn remove_game(&self, game_id: GameId) -> Vec<UserId>;
}

pub struct SpectatorServiceImpl {
    spectator_registry: Arc<ConcurrentMultiMap<UserId, GameId>>,
}

impl SpectatorServiceImpl {
    pub fn new() -> Self {
        Self {
            spectator_registry: Arc::new(ConcurrentMultiMap::new()),
        }
    }
}

impl SpectatorService for SpectatorServiceImpl {
    fn observe_game(&self, user_id: UserId, game_id: GameId) {
        self.spectator_registry.insert(user_id, game_id);
    }

    fn unobserve_game(&self, user_id: UserId, game_id: GameId) {
        self.spectator_registry.remove(&user_id, &game_id);
    }

    fn unobserve_all_games(&self, user_id: UserId) {
        self.spectator_registry.remove_by_left(&user_id);
    }

    fn get_spectators_for_game(&self, game_id: GameId) -> Vec<UserId> {
        self.spectator_registry.get_by_right(&game_id)
    }

    fn remove_game(&self, game_id: GameId) -> Vec<UserId> {
        self.spectator_registry.remove_by_right(&game_id)
    }
}
