use std::sync::Arc;

use crate::{
    domain::{PlayerId, player::PlayerRepository},
    ports::authentication::{AuthSubject, AuthenticationService},
};

#[async_trait::async_trait]
pub trait GetUsernameWorkflow {
    async fn get_username(&self, player_id: PlayerId) -> Option<String>;
}

pub struct GetUsernameWorkflowImpl<A: AuthenticationService, P: PlayerRepository> {
    authentication_service: Arc<A>,
    player_repository: Arc<P>,
}

impl<A: AuthenticationService, P: PlayerRepository> GetUsernameWorkflowImpl<A, P> {
    pub fn new(authentication_service: Arc<A>, player_repository: Arc<P>) -> Self {
        Self {
            authentication_service,
            player_repository,
        }
    }
}

#[async_trait::async_trait]
impl<A: AuthenticationService + Send + Sync + 'static, P: PlayerRepository + Send + Sync + 'static>
    GetUsernameWorkflow for GetUsernameWorkflowImpl<A, P>
{
    async fn get_username(&self, player_id: PlayerId) -> Option<String> {
        let account_id = match self.player_repository.get_player(player_id).await {
            Ok(player) => player.account_id,
            _ => return None,
        };
        let Some(account_id) = account_id else {
            return None;
        };
        let subject = self.authentication_service.get_subject(account_id)?;
        match subject.subject_type {
            AuthSubject::Player { username, .. } => Some(username),
            AuthSubject::Bot { username } => Some(username),
            AuthSubject::Guest { guest_number } => Some(format!("Guest{}", guest_number)),
        }
    }
}
