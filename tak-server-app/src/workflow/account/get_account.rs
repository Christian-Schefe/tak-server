use std::sync::Arc;

use crate::{
    domain::{PlayerId, RepoRetrieveError},
    ports::{
        authentication::{Account, AuthenticationPort},
        player_mapping::PlayerAccountMappingRepository,
    },
};

#[async_trait::async_trait]
pub trait GetAccountWorkflow {
    async fn get_account(&self, player_id: PlayerId) -> Result<Account, GetAccountError>;
}

#[derive(Debug)]
pub enum GetAccountError {
    AccountNotFound,
    RepositoryError,
}

pub struct GetAccountWorkflowImpl<A: AuthenticationPort, P: PlayerAccountMappingRepository> {
    authentication_service: Arc<A>,
    player_account_mapping_repo: Arc<P>,
}

impl<A: AuthenticationPort, P: PlayerAccountMappingRepository> GetAccountWorkflowImpl<A, P> {
    pub fn new(authentication_service: Arc<A>, player_account_mapping_repo: Arc<P>) -> Self {
        Self {
            authentication_service,
            player_account_mapping_repo,
        }
    }
}

#[async_trait::async_trait]
impl<
    A: AuthenticationPort + Send + Sync + 'static,
    P: PlayerAccountMappingRepository + Send + Sync + 'static,
> GetAccountWorkflow for GetAccountWorkflowImpl<A, P>
{
    async fn get_account(&self, player_id: PlayerId) -> Result<Account, GetAccountError> {
        let account_id = match self
            .player_account_mapping_repo
            .get_account_id(player_id)
            .await
        {
            Ok(account_id) => account_id,
            Err(RepoRetrieveError::NotFound) => return Err(GetAccountError::AccountNotFound),
            Err(RepoRetrieveError::StorageError(e)) => {
                log::error!(
                    "Failed to retrieve account ID for player {}: {}",
                    player_id,
                    e
                );
                return Err(GetAccountError::RepositoryError);
            }
        };
        match self.authentication_service.get_account(&account_id).await {
            Some(account) => Ok(account),
            None => Err(GetAccountError::AccountNotFound),
        }
    }
}
