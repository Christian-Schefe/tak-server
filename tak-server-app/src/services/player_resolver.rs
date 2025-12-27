use std::sync::Arc;

use crate::domain::{
    AccountId, PlayerId,
    player::{Player, PlayerRepository},
};

#[async_trait::async_trait]
pub trait PlayerResolverService {
    async fn resolve_player_id_by_account_id(&self, account_id: AccountId) -> Result<PlayerId, ()>;
    async fn resolve_account_id_by_player_id(&self, player_id: PlayerId) -> Result<AccountId, ()>;
}

pub struct PlayerResolverServiceImpl<PR: PlayerRepository> {
    player_repository: Arc<PR>,
}

impl<PR: PlayerRepository> PlayerResolverServiceImpl<PR> {
    pub fn new(player_repository: std::sync::Arc<PR>) -> Self {
        Self { player_repository }
    }
}

#[async_trait::async_trait]
impl<PR: PlayerRepository + Send + Sync + 'static> PlayerResolverService
    for PlayerResolverServiceImpl<PR>
{
    async fn resolve_player_id_by_account_id(&self, account_id: AccountId) -> Result<PlayerId, ()> {
        match self
            .player_repository
            .get_or_create_player_by_account_id(account_id, move || Player::new(Some(account_id)))
            .await
        {
            Ok(player) => Ok(player.player_id),
            Err(_) => Err(()),
        }
    }
    async fn resolve_account_id_by_player_id(&self, player_id: PlayerId) -> Result<AccountId, ()> {
        match self.player_repository.get_player(player_id).await {
            Ok(player) => match player.account_id {
                Some(account_id) => Ok(account_id),
                None => Err(()),
            },
            Err(_) => Err(()),
        }
    }
}
