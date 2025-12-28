use std::sync::Arc;

use crate::{
    domain::{
        MatchId, PlayerId,
        r#match::{MatchService, RequestRematchError},
    },
    workflow::matchmaking::create_game::CreateGameFromMatchWorkflow,
};

#[async_trait::async_trait]
pub trait RematchUseCase {
    async fn request_or_accept_rematch(
        &self,
        match_id: MatchId,
        player: PlayerId,
    ) -> Result<(), RequestRematchError>;
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
    ) -> Result<(), RequestRematchError> {
        let should_create_game = self
            .match_service
            .request_or_accept_rematch(match_id, player)?;
        if should_create_game {
            let Some(match_entry) = self.match_service.get_match(match_id) else {
                return Err(RequestRematchError::MatchNotFound);
            };
            self.create_game_workflow
                .create_game_from_match(&match_entry)
                .await;
        }
        Ok(())
    }
}
