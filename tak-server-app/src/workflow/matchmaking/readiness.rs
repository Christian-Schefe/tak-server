use std::sync::Arc;

use crate::{
    domain::{
        MatchId, PlayerId, RepoRetrieveError,
        matches::{Match, MatchReadinessService, MatchRepository, MatchStatus},
    },
    ports::notification::{ListenerMatchEventType, ListenerMessage},
    workflow::{
        matchmaking::{MatchReadinessStatus, create_game::CreateGameFromMatchWorkflow},
        player::notify_player::NotifyPlayerWorkflow,
    },
};

#[async_trait::async_trait]
pub trait MatchReadinessUseCase {
    fn get_readiness_status(&self, match_id: MatchId) -> MatchReadinessStatus;
    async fn set_player_ready(
        &self,
        match_id: MatchId,
        player: PlayerId,
    ) -> Result<(), MatchReadinessError>;
    async fn set_player_not_ready(
        &self,
        match_id: MatchId,
        player: PlayerId,
    ) -> Result<(), MatchReadinessError>;
}

pub struct MatchReadinessUseCaseImpl<
    M: MatchRepository,
    C: CreateGameFromMatchWorkflow,
    RS: MatchReadinessService,
    L: NotifyPlayerWorkflow,
> {
    match_repo: Arc<M>,
    create_game_workflow: Arc<C>,
    match_readiness_service: Arc<RS>,
    notification_port: Arc<L>,
}

impl<
    M: MatchRepository,
    C: CreateGameFromMatchWorkflow,
    RS: MatchReadinessService,
    L: NotifyPlayerWorkflow,
> MatchReadinessUseCaseImpl<M, C, RS, L>
{
    pub fn new(
        match_repo: Arc<M>,
        create_game_workflow: Arc<C>,
        match_readiness_service: Arc<RS>,
        notification_port: Arc<L>,
    ) -> Self {
        Self {
            match_repo,
            create_game_workflow,
            match_readiness_service,
            notification_port,
        }
    }

    async fn get_match(&self, match_id: MatchId) -> Result<Match, MatchReadinessError> {
        let match_entry = self
            .match_repo
            .get_match(match_id)
            .await
            .map_err(|e| match e {
                RepoRetrieveError::NotFound => {
                    tracing::error!("Match not found for match ID {}: {}", match_id, e);
                    MatchReadinessError::MatchNotFound
                }
                RepoRetrieveError::StorageError(err) => {
                    tracing::error!(
                        "Database error while retrieving match {}: {}",
                        match_id,
                        err
                    );
                    MatchReadinessError::Internal
                }
            })?;
        Ok(match_entry)
    }
}

#[derive(Debug)]
pub enum MatchReadinessError {
    MatchNotFound,
    Internal,
}

#[async_trait::async_trait]
impl<
    M: MatchRepository + Send + Sync + 'static,
    C: CreateGameFromMatchWorkflow + Send + Sync + 'static,
    RS: MatchReadinessService + Send + Sync + 'static,
    L: NotifyPlayerWorkflow + Send + Sync + 'static,
> MatchReadinessUseCase for MatchReadinessUseCaseImpl<M, C, RS, L>
{
    fn get_readiness_status(&self, match_id: MatchId) -> MatchReadinessStatus {
        MatchReadinessStatus {
            player_ready: self.match_readiness_service.get_readiness_status(match_id),
        }
    }

    #[tracing::instrument(skip(self), ret, err(Debug))]
    async fn set_player_ready(
        &self,
        match_id: MatchId,
        player: PlayerId,
    ) -> Result<(), MatchReadinessError> {
        let match_entry = self.get_match(match_id).await?;
        if match_entry.player1 != player && match_entry.player2 != player {
            tracing::error!(
                "Player {} is not a participant in match {}",
                player,
                match_id
            );
            return Err(MatchReadinessError::MatchNotFound);
        }
        let MatchStatus::Waiting = match_entry.status else {
            tracing::error!(
                "Match {} is not in a state that allows readiness. Current status: {:?}",
                match_id,
                match_entry.status
            );
            return Err(MatchReadinessError::MatchNotFound);
        };
        let should_create_game = self
            .match_readiness_service
            .set_player_ready(match_id, player);
        if should_create_game {
            if let Err(e) = self
                .create_game_workflow
                .create_game_from_match(match_id)
                .await
            {
                tracing::error!("Failed to create game from match {}: {:?}", match_id, e);
                return Err(MatchReadinessError::Internal);
            }
        } else {
            let msg = ListenerMessage::MatchEvent {
                match_id,
                event_type: ListenerMatchEventType::MatchReadinessChanged {
                    player_id: Some(player),
                },
            };

            self.notification_port
                .notify_players(&[match_entry.player1, match_entry.player2], &msg)
                .await;
        }
        Ok(())
    }

    #[tracing::instrument(skip(self), ret, err(Debug))]
    async fn set_player_not_ready(
        &self,
        match_id: MatchId,
        player: PlayerId,
    ) -> Result<(), MatchReadinessError> {
        let match_entry = self.get_match(match_id).await?;
        if match_entry.player1 != player && match_entry.player2 != player {
            tracing::error!(
                "Player {} is not a participant in match {}",
                player,
                match_id
            );
            return Err(MatchReadinessError::MatchNotFound);
        }
        let MatchStatus::Waiting = match_entry.status else {
            tracing::error!(
                "Match {} is not in a state that allows retracting readiness. Current status: {:?}",
                match_id,
                match_entry.status
            );
            return Err(MatchReadinessError::Internal);
        };
        let did_remove = self
            .match_readiness_service
            .set_player_not_ready(match_id, player);
        if did_remove {
            let msg = ListenerMessage::MatchEvent {
                match_id,
                event_type: ListenerMatchEventType::MatchReadinessChanged { player_id: None },
            };
            self.notification_port
                .notify_players(&[match_entry.player1, match_entry.player2], &msg)
                .await;
        }
        Ok(())
    }
}
