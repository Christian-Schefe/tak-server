use std::sync::Arc;

use crate::{
    domain::{AccountId, PlayerId, player::Player},
    ports::player_mapping::PlayerAccountMappingRepository,
};

#[async_trait::async_trait]
pub trait PlayerResolverService {
    async fn resolve_player_id_by_account_id(
        &self,
        account_id: &AccountId,
    ) -> Result<PlayerId, ResolveError>;
    async fn resolve_account_id_by_player_id(
        &self,
        player_id: PlayerId,
    ) -> Result<AccountId, ResolveError>;
}

pub enum ResolveError {
    Internal,
}

pub struct PlayerResolverServiceImpl<PAM: PlayerAccountMappingRepository> {
    player_account_mapping_repository: Arc<PAM>,
}

impl<PAM: PlayerAccountMappingRepository> PlayerResolverServiceImpl<PAM> {
    pub fn new(player_account_mapping_repository: Arc<PAM>) -> Self {
        Self {
            player_account_mapping_repository,
        }
    }
}

#[async_trait::async_trait]
impl<PAM: PlayerAccountMappingRepository + Send + Sync + 'static> PlayerResolverService
    for PlayerResolverServiceImpl<PAM>
{
    async fn resolve_player_id_by_account_id(
        &self,
        account_id: &AccountId,
    ) -> Result<PlayerId, ResolveError> {
        match self
            .player_account_mapping_repository
            .get_or_create_player_id(account_id, || Player::new().player_id)
            .await
        {
            Ok(player_id) => Ok(player_id),
            Err(e) => {
                log::error!("Failed to resolve player id by account id: {}", e);
                Err(ResolveError::Internal)
            }
        }
    }

    async fn resolve_account_id_by_player_id(
        &self,
        player_id: PlayerId,
    ) -> Result<AccountId, ResolveError> {
        match self
            .player_account_mapping_repository
            .get_account_id(player_id)
            .await
        {
            Ok(account_id) => Ok(account_id),
            Err(e) => {
                log::error!("Failed to resolve account id by player id: {}", e);
                Err(ResolveError::Internal)
            }
        }
    }
}
