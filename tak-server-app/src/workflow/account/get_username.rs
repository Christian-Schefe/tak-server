use std::sync::Arc;

use crate::{
    domain::{PlayerId, player::PlayerRepository},
    ports::authentication::{AuthSubject, AuthenticationPort},
};

#[async_trait::async_trait]
pub trait GetUsernameWorkflow {
    async fn get_username(&self, player_id: PlayerId) -> Option<String>;
}

pub struct GetUsernameWorkflowImpl<A: AuthenticationPort, P: PlayerRepository> {
    authentication_service: Arc<A>,
    player_repository: Arc<P>,
}

impl<A: AuthenticationPort, P: PlayerRepository> GetUsernameWorkflowImpl<A, P> {
    pub fn new(authentication_service: Arc<A>, player_repository: Arc<P>) -> Self {
        Self {
            authentication_service,
            player_repository,
        }
    }
}

#[async_trait::async_trait]
impl<A: AuthenticationPort + Send + Sync + 'static, P: PlayerRepository + Send + Sync + 'static>
    GetUsernameWorkflow for GetUsernameWorkflowImpl<A, P>
{
    async fn get_username(&self, player_id: PlayerId) -> Option<String> {
        let player = match self.player_repository.get_player(player_id).await {
            Ok(player) => player,
            _ => return None,
        };
        let Some(account_id) = player.account_id else {
            return None;
        };
        let account = self.authentication_service.get_account(account_id).await?;
        match account.subject_type {
            AuthSubject::Player { username, .. } => Some(username),
            AuthSubject::Guest { guest_number } => Some(format!("Guest{}", guest_number)),
        }
    }
}
