use std::sync::Arc;

use dashmap::DashMap;
use uuid::Uuid;

use crate::domain::{AccountId, PlayerId};

pub struct Player {
    pub player_id: PlayerId,
    pub is_silenced: bool,
    pub is_banned: bool,
    pub is_bot: bool,
}

impl Player {
    pub fn new() -> Self {
        Self {
            player_id: PlayerId(Uuid::new_v4()),
            is_silenced: false,
            is_banned: false,
            is_bot: false,
        }
    }
}

pub trait PlayerRepository {
    fn create_player(
        &self,
        player: Player,
        account_id: Option<AccountId>,
    ) -> Result<(), CreatePlayerError>;
    fn delete_player(&self, player_id: PlayerId);
    fn get_player(&self, player_id: PlayerId) -> Option<Player>;
    fn link_account(&self, player_id: PlayerId, account_id: AccountId);
    fn unlink_account(&self, player_id: PlayerId);
    fn get_player_by_account_id(&self, account_id: AccountId) -> Option<Player>;
    fn get_account_for_player(&self, player_id: PlayerId) -> Option<AccountId>;
    fn set_player_silenced(&self, player_id: PlayerId, silenced: bool);
    fn set_player_banned(&self, player_id: PlayerId, banned: bool);
    fn set_player_is_bot(&self, player_id: PlayerId, is_bot: bool);
}

pub enum CreatePlayerError {
    PlayerAlreadyExists,
    StorageError,
}

pub trait PlayerService {
    fn set_player_online(&self, player_id: PlayerId) -> Option<Vec<PlayerId>>;
    fn set_player_offline(&self, player_id: PlayerId) -> Option<Vec<PlayerId>>;
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

    fn get_players_online(&self) -> Vec<PlayerId> {
        self.online_players
            .iter()
            .map(|entry| *entry.key())
            .collect()
    }
}

impl PlayerService for PlayerServiceImpl {
    fn set_player_online(&self, player_id: PlayerId) -> Option<Vec<PlayerId>> {
        if self.online_players.insert(player_id, ()).is_none() {
            // Only return updated list if the player was not already online
            return Some(self.get_players_online());
        }
        None
    }

    fn set_player_offline(&self, player_id: PlayerId) -> Option<Vec<PlayerId>> {
        if self.online_players.remove(&player_id).is_some() {
            // Only return updated list if the player was actually online
            return Some(self.get_players_online());
        }
        None
    }
}
