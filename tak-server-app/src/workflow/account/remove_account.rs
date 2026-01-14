use std::sync::Arc;

use crate::{
    domain::{AccountId, RepoError, stats::StatsRepository},
    ports::player_mapping::PlayerAccountMappingRepository,
};

#[async_trait::async_trait]
pub trait RemoveAccountWorkflow {
    async fn remove_account(&self, account_id: &AccountId) -> Result<(), RemoveAccountError>;
}

pub struct RemoveAccountWorkflowImpl<R: PlayerAccountMappingRepository, S: StatsRepository> {
    player_account_mapping_repo: Arc<R>,
    stats_repo: Arc<S>,
}

impl<R: PlayerAccountMappingRepository, S: StatsRepository> RemoveAccountWorkflowImpl<R, S> {
    pub fn new(player_account_mapping_repo: Arc<R>, stats_repo: Arc<S>) -> Self {
        Self {
            player_account_mapping_repo,
            stats_repo,
        }
    }
}

#[derive(Debug)]
pub enum RemoveAccountError {
    PlayerMappingRemovalFailed,
    StatsRemovalFailed,
}

#[async_trait::async_trait]
impl<
    R: PlayerAccountMappingRepository + Send + Sync + 'static,
    S: StatsRepository + Send + Sync + 'static,
> RemoveAccountWorkflow for RemoveAccountWorkflowImpl<R, S>
{
    async fn remove_account(&self, account_id: &AccountId) -> Result<(), RemoveAccountError> {
        match self
            .player_account_mapping_repo
            .remove_account_id(account_id)
            .await
        {
            Ok(Some(player_id)) => {
                log::info!("Removed player mapping for account_id {:?}", account_id);
                match self.stats_repo.remove_player_stats(player_id).await {
                    Ok(()) => {
                        log::info!("Removed stats for player_id {:?}", player_id);
                        Ok(())
                    }
                    Err(RepoError::StorageError(e)) => {
                        log::error!(
                            "Failed to remove stats for player_id {:?}: {:?}",
                            player_id,
                            e
                        );
                        Err(RemoveAccountError::StatsRemovalFailed)
                    }
                }
            }
            Ok(None) => {
                log::info!(
                    "No player mapping found for account_id {:?}, nothing to remove",
                    account_id
                );
                Ok(())
            }
            Err(RepoError::StorageError(e)) => {
                log::error!(
                    "Failed to remove account mapping for account_id {:?}: {:?}",
                    account_id,
                    e
                );
                Err(RemoveAccountError::PlayerMappingRemovalFailed)
            }
        }
    }
}
