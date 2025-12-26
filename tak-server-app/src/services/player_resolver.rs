use std::sync::Arc;

use crate::domain::{
    AccountId, PlayerId,
    player::{Player, PlayerRepository},
};

#[async_trait::async_trait]
pub trait PlayerResolver {
    async fn resolve_player_by_account_id(&self, account_id: AccountId) -> Result<PlayerId, ()>;
}

pub struct PlayerResolverImpl<PR: PlayerRepository> {
    player_repository: Arc<PR>,
}

impl<PR: PlayerRepository> PlayerResolverImpl<PR> {
    pub fn new(player_repository: std::sync::Arc<PR>) -> Self {
        Self { player_repository }
    }
}

#[async_trait::async_trait]
impl<PR: PlayerRepository + Send + Sync + 'static> PlayerResolver for PlayerResolverImpl<PR> {
    async fn resolve_player_by_account_id(&self, account_id: AccountId) -> Result<PlayerId, ()> {
        match self
            .player_repository
            .get_or_create_player_by_account_id(account_id, move || Player::new(Some(account_id)))
            .await
        {
            Ok(player) => Ok(player.player_id),
            Err(_) => Err(()),
        }
    }
}
