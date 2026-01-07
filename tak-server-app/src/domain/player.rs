use std::sync::Arc;

use dashmap::DashMap;

use crate::domain::PlayerId;

pub struct Player {
    pub player_id: PlayerId,
}

impl Player {
    pub fn new() -> Self {
        Self {
            player_id: PlayerId(uuid::Uuid::new_v4()),
        }
    }
}

pub trait PlayerService {
    fn set_player_online(&self, player_id: PlayerId) -> Option<Vec<PlayerId>>;
    fn set_player_offline(&self, player_id: PlayerId) -> Option<Vec<PlayerId>>;
    fn get_online_players(&self) -> Vec<PlayerId>;
}

pub struct PlayerServiceImpl {
    online_players: Arc<DashMap<PlayerId, ()>>,
}

impl PlayerServiceImpl {
    pub fn new() -> Self {
        Self {
            online_players: Arc::new(DashMap::new()),
        }
    }
}

impl PlayerService for PlayerServiceImpl {
    fn set_player_online(&self, player_id: PlayerId) -> Option<Vec<PlayerId>> {
        if self.online_players.insert(player_id, ()).is_none() {
            // Only return updated list if the player was not already online
            return Some(self.get_online_players());
        }
        None
    }

    fn set_player_offline(&self, player_id: PlayerId) -> Option<Vec<PlayerId>> {
        if self.online_players.remove(&player_id).is_some() {
            // Only return updated list if the player was actually online
            return Some(self.get_online_players());
        }
        None
    }

    fn get_online_players(&self) -> Vec<PlayerId> {
        self.online_players
            .iter()
            .map(|entry| *entry.key())
            .collect()
    }
}
