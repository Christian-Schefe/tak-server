use std::sync::Arc;

use crate::{
    domain::{
        MatchId, PlayerId, RepoRetrieveError,
        matches::{Match, MatchRepository, MatchStatus, RematchService},
    },
    ports::notification::{ListenerMatchEventType, ListenerMessage},
    workflow::{
        matchmaking::{RematchStatus, create_game::CreateGameFromMatchWorkflow},
        player::notify_player::NotifyPlayerWorkflow,
    },
};

#[async_trait::async_trait]
pub trait RematchUseCase {
    fn get_rematch_status(&self, match_id: MatchId) -> RematchStatus;
    async fn request_or_accept_rematch(
        &self,
        match_id: MatchId,
        player: PlayerId,
    ) -> Result<(), RematchError>;
    async fn retract_rematch_request(
        &self,
        match_id: MatchId,
        player: PlayerId,
    ) -> Result<(), RematchError>;
}

pub struct RematchUseCaseImpl<
    M: MatchRepository,
    C: CreateGameFromMatchWorkflow,
    RS: RematchService,
    L: NotifyPlayerWorkflow,
> {
    match_repo: Arc<M>,
    create_game_workflow: Arc<C>,
    rematch_service: Arc<RS>,
    notification_port: Arc<L>,
}

impl<
    M: MatchRepository,
    C: CreateGameFromMatchWorkflow,
    RS: RematchService,
    L: NotifyPlayerWorkflow,
> RematchUseCaseImpl<M, C, RS, L>
{
    pub fn new(
        match_repo: Arc<M>,
        create_game_workflow: Arc<C>,
        rematch_service: Arc<RS>,
        notification_port: Arc<L>,
    ) -> Self {
        Self {
            match_repo,
            create_game_workflow,
            rematch_service,
            notification_port,
        }
    }

    async fn get_match(&self, match_id: MatchId) -> Result<Match, RematchError> {
        let match_entry = self
            .match_repo
            .get_match(match_id)
            .await
            .map_err(|e| match e {
                RepoRetrieveError::NotFound => {
                    tracing::error!("Match not found for match ID {}: {}", match_id, e);
                    RematchError::MatchNotFound
                }
                RepoRetrieveError::StorageError(err) => {
                    tracing::error!(
                        "Database error while retrieving match {}: {}",
                        match_id,
                        err
                    );
                    RematchError::Internal
                }
            })?;
        Ok(match_entry)
    }
}

#[derive(Debug)]
pub enum RematchError {
    MatchNotFound,
    Internal,
}

#[async_trait::async_trait]
impl<
    M: MatchRepository + Send + Sync + 'static,
    C: CreateGameFromMatchWorkflow + Send + Sync + 'static,
    RS: RematchService + Send + Sync + 'static,
    L: NotifyPlayerWorkflow + Send + Sync + 'static,
> RematchUseCase for RematchUseCaseImpl<M, C, RS, L>
{
    fn get_rematch_status(&self, match_id: MatchId) -> RematchStatus {
        RematchStatus {
            rematch_requested_by: self.rematch_service.get_rematch_status(match_id),
        }
    }

    #[tracing::instrument(skip(self), ret, err(Debug))]
    async fn request_or_accept_rematch(
        &self,
        match_id: MatchId,
        player: PlayerId,
    ) -> Result<(), RematchError> {
        let match_entry = self.get_match(match_id).await?;
        if match_entry.player1 != player && match_entry.player2 != player {
            tracing::error!(
                "Player {} is not a participant in match {}",
                player,
                match_id
            );
            return Err(RematchError::MatchNotFound);
        }
        let MatchStatus::Waiting = match_entry.status else {
            tracing::error!(
                "Match {} is not in a state that allows rematches. Current status: {:?}",
                match_id,
                match_entry.status
            );
            return Err(RematchError::MatchNotFound);
        };
        let should_create_game = self
            .rematch_service
            .request_or_accept_rematch(match_id, player);
        if should_create_game {
            if let Err(e) = self
                .create_game_workflow
                .create_game_from_match(match_id)
                .await
            {
                tracing::error!("Failed to create game from match {}: {:?}", match_id, e);
                return Err(RematchError::Internal);
            }
        } else {
            let msg = ListenerMessage::MatchEvent {
                match_id,
                event_type: ListenerMatchEventType::MatchRematchRequestAdded {
                    requesting_player_id: player,
                },
            };

            self.notification_port
                .notify_players(&[match_entry.player1, match_entry.player2], &msg)
                .await;
        }
        Ok(())
    }

    #[tracing::instrument(skip(self), ret, err(Debug))]
    async fn retract_rematch_request(
        &self,
        match_id: MatchId,
        player: PlayerId,
    ) -> Result<(), RematchError> {
        let match_entry = self.get_match(match_id).await?;
        if match_entry.player1 != player && match_entry.player2 != player {
            tracing::error!(
                "Player {} is not a participant in match {}",
                player,
                match_id
            );
            return Err(RematchError::MatchNotFound);
        }
        let MatchStatus::Waiting = match_entry.status else {
            tracing::error!(
                "Match {} is not in a state that allows retracting rematch requests. Current status: {:?}",
                match_id,
                match_entry.status
            );
            return Err(RematchError::Internal);
        };
        let did_remove = self
            .rematch_service
            .retract_rematch_request(match_id, player);
        if did_remove {
            let msg = ListenerMessage::MatchEvent {
                match_id,
                event_type: ListenerMatchEventType::MatchRematchRequestRemoved,
            };
            self.notification_port
                .notify_players(&[match_entry.player1, match_entry.player2], &msg)
                .await;
        }
        Ok(())
    }
}
