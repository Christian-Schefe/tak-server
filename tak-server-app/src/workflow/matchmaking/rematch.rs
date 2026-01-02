use std::sync::Arc;

use crate::{
    domain::{
        GameId, PlayerId,
        r#match::{MatchService, RequestRematchError},
    },
    workflow::matchmaking::create_game::CreateGameFromMatchWorkflow,
};

#[async_trait::async_trait]
pub trait RematchUseCase {
    async fn request_or_accept_rematch(
        &self,
        game_id: GameId,
        player: PlayerId,
    ) -> Result<(), RequestOrAcceptRematchError>;
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

pub enum RequestOrAcceptRematchError {
    MatchNotFound,
    RequestRematchError(RequestRematchError),
    FailedToCreateGame,
}

#[async_trait::async_trait]
impl<
    M: MatchService + Send + Sync + 'static,
    C: CreateGameFromMatchWorkflow + Send + Sync + 'static,
> RematchUseCase for RematchUseCaseImpl<M, C>
{
    async fn request_or_accept_rematch(
        &self,
        game_id: GameId,
        player: PlayerId,
    ) -> Result<(), RequestOrAcceptRematchError> {
        let match_id = self
            .match_service
            .get_match_id_by_game_id(game_id)
            .ok_or(RequestOrAcceptRematchError::MatchNotFound)?;
        let should_create_game = match self
            .match_service
            .request_or_accept_rematch(match_id, player)
        {
            Ok(should_create) => should_create,
            Err(e) => {
                log::error!(
                    "Failed to request or accept rematch for match {}: {:?}",
                    match_id,
                    e
                );
                return Err(RequestOrAcceptRematchError::RequestRematchError(e));
            }
        };
        if should_create_game {
            if let Err(e) = self
                .create_game_workflow
                .create_game_from_match(match_id)
                .await
            {
                log::error!("Failed to create game from match {}: {:?}", match_id, e);
                return Err(RequestOrAcceptRematchError::FailedToCreateGame);
            }
        }
        Ok(())
    }
}
