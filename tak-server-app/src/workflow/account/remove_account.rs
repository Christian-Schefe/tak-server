use std::sync::Arc;

use crate::{domain::AccountId, ports::player_mapping::PlayerAccountMappingRepository};

#[async_trait::async_trait]
pub trait RemoveAccountWorkflow {
    async fn remove_account(&self, account_id: &AccountId) -> Result<(), RemoveAccountError>;
}

pub struct RemoveAccountWorkflowImpl<R: PlayerAccountMappingRepository> {
    player_account_mapping_repo: Arc<R>,
}

impl<R> RemoveAccountWorkflowImpl<R>
where
    R: PlayerAccountMappingRepository + Send + Sync + 'static,
{
    pub fn new(player_account_mapping_repo: Arc<R>) -> Self {
        Self {
            player_account_mapping_repo,
        }
    }
}

#[derive(Debug)]
pub enum RemoveAccountError {
    PlayerMappingRemovalFailed,
}

#[async_trait::async_trait]
impl<R> RemoveAccountWorkflow for RemoveAccountWorkflowImpl<R>
where
    R: PlayerAccountMappingRepository + Send + Sync + 'static,
{
    async fn remove_account(&self, account_id: &AccountId) -> Result<(), RemoveAccountError> {
        match self
            .player_account_mapping_repo
            .remove_account_id(account_id)
            .await
        {
            Ok(()) => {
                log::info!("Removed player mapping for account_id {:?}", account_id,);
                Ok(())
            }
            Err(e) => {
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
