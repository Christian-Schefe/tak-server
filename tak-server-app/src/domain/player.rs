use std::sync::Arc;

use dashmap::DashMap;
use uuid::Uuid;

use crate::domain::{AccountId, PlayerId};

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[async_trait::async_trait]
pub trait PlayerRepository {
    async fn create_player(
        &self,
        player: Player,
        account_id: Option<AccountId>,
    ) -> Result<(), CreatePlayerError>;
    async fn delete_player(&self, player_id: PlayerId) -> Result<bool, PlayerRepoError>;
    async fn get_player(&self, player_id: PlayerId) -> Result<Option<Player>, PlayerRepoError>;
    async fn link_account(
        &self,
        player_id: PlayerId,
        account_id: AccountId,
    ) -> Result<(), PlayerRepoError>;
    async fn unlink_account(&self, player_id: PlayerId) -> Result<(), PlayerRepoError>;
    async fn get_player_by_account_id(
        &self,
        account_id: AccountId,
    ) -> Result<Option<Player>, PlayerRepoError>;
    async fn get_account_id_for_player(&self, player_id: PlayerId) -> Option<AccountId>;
    async fn set_player_silenced(&self, player_id: PlayerId, silenced: bool);
    async fn set_player_banned(&self, player_id: PlayerId, banned: bool);
    async fn set_player_is_bot(&self, player_id: PlayerId, is_bot: bool);
}

pub enum CreatePlayerError {
    PlayerAlreadyExists,
    StorageError(String),
}

pub enum PlayerRepoError {
    StorageError(String),
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
