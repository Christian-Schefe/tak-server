use std::sync::Arc;

use crate::{
    domain::{MatchId, PlayerId, r#match::MatchService},
    workflow::matchmaking::create_game::CreateGameFromMatchWorkflow,
};

#[async_trait::async_trait]
pub trait RematchUseCase {
    async fn request_or_accept_rematch(
        &self,
        match_id: MatchId,
        player: PlayerId,
    ) -> Result<(), RematchError>;
}

pub struct RematchUseCaseImpl<M: MatchService, C: CreateGameFromMatchWorkflow> {
    match_service: Arc<M>,
    create_game_workflow: Arc<C>,
}

impl<M: MatchService, C: CreateGameFromMatchWorkflow> RematchUseCaseImpl<M, C> {
    pub fn new(match_service: Arc<M>, create_game_workflow: Arc<C>) -> Self {
        Self {
            match_service,
            create_game_workflow,
        }
    }
}

pub enum RematchError {
    MatchNotFound,
    NotParticipant,
    RematchAlreadyAccepted,
}

#[async_trait::async_trait]
impl<
    M: MatchService + Send + Sync + 'static,
    C: CreateGameFromMatchWorkflow + Send + Sync + 'static,
> RematchUseCase for RematchUseCaseImpl<M, C>
{
    async fn request_or_accept_rematch(
        &self,
        match_id: MatchId,
        player: PlayerId,
    ) -> Result<(), RematchError> {
        let should_create_game = match self
            .match_service
            .request_or_accept_rematch(match_id, player)
        {
            Ok(s) => s,
            Err(crate::domain::r#match::RematchError::MatchNotFound) => {
                return Err(RematchError::MatchNotFound);
            }
            Err(crate::domain::r#match::RematchError::InvalidPlayer) => {
                return Err(RematchError::NotParticipant);
            }
        };
        if should_create_game {
            let Some(match_entry) = self.match_service.get_match(match_id) else {
                return Err(RematchError::MatchNotFound);
            };
            self.create_game_workflow
                .create_game_from_match(&match_entry)
                .await;
        }
        Ok(())
    }
}
