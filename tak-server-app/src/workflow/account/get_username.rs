use std::sync::Arc;

use crate::{
    domain::PlayerId,
    ports::{authentication::AuthenticationPort, player_mapping::PlayerAccountMappingRepository},
};

#[async_trait::async_trait]
pub trait GetUsernameWorkflow {
    async fn get_username(&self, player_id: PlayerId) -> Option<String>;
}

pub struct GetUsernameWorkflowImpl<A: AuthenticationPort, P: PlayerAccountMappingRepository> {
    authentication_service: Arc<A>,
    player_account_mapping_repo: Arc<P>,
}

impl<A: AuthenticationPort, P: PlayerAccountMappingRepository> GetUsernameWorkflowImpl<A, P> {
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
> GetUsernameWorkflow for GetUsernameWorkflowImpl<A, P>
{
    async fn get_username(&self, player_id: PlayerId) -> Option<String> {
        let account_id = match self
            .player_account_mapping_repo
            .get_account_id(player_id)
            .await
        {
            Ok(account_id) => account_id,
            _ => return None,
        };
        let account = self.authentication_service.get_account(account_id).await?;
        Some(account.username)
    }
}
