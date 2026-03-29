use std::sync::Arc;

use crate::{
    domain::{
        MatchId, PlayerId, RepoRetrieveError,
        matches::{MatchRepository, RematchService, RequestRematchError},
    },
    workflow::matchmaking::create_game::CreateGameFromMatchWorkflow,
};

#[async_trait::async_trait]
pub trait RematchUseCase {
    async fn request_or_accept_rematch(
        &self,
        match_id: MatchId,
        player: PlayerId,
    ) -> Result<(), RequestOrAcceptRematchError>;
}

pub struct RematchUseCaseImpl<
    M: MatchRepository,
    C: CreateGameFromMatchWorkflow,
    RS: RematchService,
> {
    match_repo: Arc<M>,
    create_game_workflow: Arc<C>,
    rematch_service: Arc<RS>,
}

impl<M: MatchRepository, C: CreateGameFromMatchWorkflow, RS: RematchService>
    RematchUseCaseImpl<M, C, RS>
{
    pub fn new(match_repo: Arc<M>, create_game_workflow: Arc<C>, rematch_service: Arc<RS>) -> Self {
        Self {
            match_repo,
            create_game_workflow,
            rematch_service,
        }
    }
}

pub enum RequestOrAcceptRematchError {
    MatchNotFound,
    RequestRematchError(RequestRematchError),
    FailedToCreateGame,
    RepositoryError,
}

#[async_trait::async_trait]
impl<
    M: MatchRepository + Send + Sync + 'static,
    C: CreateGameFromMatchWorkflow + Send + Sync + 'static,
    RS: RematchService + Send + Sync + 'static,
> RematchUseCase for RematchUseCaseImpl<M, C, RS>
{
    async fn request_or_accept_rematch(
        &self,
        match_id: MatchId,
        player: PlayerId,
    ) -> Result<(), RequestOrAcceptRematchError> {
        let match_entry = self
            .match_repo
            .get_match(match_id)
            .await
            .map_err(|e| match e {
                RepoRetrieveError::NotFound => {
                    tracing::error!("Match not found for match ID {}: {}", match_id, e);
                    RequestOrAcceptRematchError::MatchNotFound
                }
                RepoRetrieveError::StorageError(err) => {
                    tracing::error!(
                        "Database error while retrieving match {}: {}",
                        match_id,
                        err
                    );
                    RequestOrAcceptRematchError::RepositoryError
                }
            })?;
        if match_entry.player1 != player && match_entry.player2 != player {
            tracing::error!(
                "Player {} is not a participant in match {}",
                player,
                match_id
            );
            return Err(RequestOrAcceptRematchError::MatchNotFound);
        }
        let should_create_game = self
            .rematch_service
            .request_or_accept_rematch(match_id, player)
            .map_err(|e| {
                tracing::error!(
                    "Failed to request or accept rematch for match {}: {:?}",
                    match_id,
                    e
                );
                RequestOrAcceptRematchError::RequestRematchError(e)
            })?;
        if should_create_game {
            if let Err(e) = self
                .create_game_workflow
                .create_game_from_match(match_id)
                .await
            {
                tracing::error!("Failed to create game from match {}: {:?}", match_id, e);
                return Err(RequestOrAcceptRematchError::FailedToCreateGame);
            }
        }
        Ok(())
    }
}
