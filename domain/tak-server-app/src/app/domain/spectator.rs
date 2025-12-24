use std::sync::Arc;

use more_dashmap::many_many::ManyManyDashMap;

use crate::app::domain::{GameId, ListenerId};

pub trait SpectatorService {
    fn observe_game(&self, game_id: GameId, listener_id: ListenerId);
    fn unobserve_game(&self, game_id: GameId, listener_id: ListenerId);
    fn unobserve_all_games(&self, listener_id: ListenerId);
}

pub struct SpectatorServiceImpl {
    game_spectators: Arc<ManyManyDashMap<GameId, ListenerId>>,
}

impl SpectatorServiceImpl {
    pub fn new() -> Self {
        Self {
            game_spectators: Arc::new(ManyManyDashMap::new()),
        }
    }
}

impl SpectatorService for SpectatorServiceImpl {
    fn observe_game(&self, game_id: GameId, listener_id: ListenerId) {
        self.game_spectators.insert(game_id, listener_id);
    }

    fn unobserve_game(&self, game_id: GameId, listener_id: ListenerId) {
        self.game_spectators.remove(&game_id, &listener_id);
    }

    fn unobserve_all_games(&self, listener_id: ListenerId) {
        self.game_spectators.remove_value(&listener_id);
    }
}
