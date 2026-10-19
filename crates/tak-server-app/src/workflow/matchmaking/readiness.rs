use std::sync::Arc;

use crate::{
    PortsCollection,
    ports::{
        matches::MatchRepository,
        notification::{ListenerMatchEventType, ListenerMessage},
        transaction::{TransactionError, TransactionManager},
    },
    workflow::{
        matchmaking::{MatchReadinessStatus, create_game::CreateGameFromMatchWorkflow},
        player::notify_player::NotifyPlayerWorkflow,
    },
};
use tak_domain::{
    MatchId, UserId,
    matches::{Match, MatchReadinessService, MatchStatus},
};

#[async_trait::async_trait]
pub trait MatchReadinessUseCase {
    fn get_readiness_status(&self, match_id: MatchId) -> MatchReadinessStatus;
    async fn set_player_ready(
        &self,
        match_id: MatchId,
        user_id: UserId,
    ) -> Result<(), MatchReadinessError>;
    async fn set_player_not_ready(
        &self,
        match_id: MatchId,
        user_id: UserId,
    ) -> Result<(), MatchReadinessError>;
}

pub struct MatchReadinessUseCaseImpl<
    Ports: PortsCollection,
    C: CreateGameFromMatchWorkflow,
    RS: MatchReadinessService,
    L: NotifyPlayerWorkflow,
> {
    ports: Arc<Ports>,
    create_game_workflow: Arc<C>,
    match_readiness_service: Arc<RS>,
    notification_port: Arc<L>,
}

impl<
    Ports: PortsCollection,
    C: CreateGameFromMatchWorkflow,
    RS: MatchReadinessService,
    L: NotifyPlayerWorkflow,
> MatchReadinessUseCaseImpl<Ports, C, RS, L>
{
    pub fn new(
        ports: Arc<Ports>,
        create_game_workflow: Arc<C>,
        match_readiness_service: Arc<RS>,
        notification_port: Arc<L>,
    ) -> Self {
        Self {
            ports,
            create_game_workflow,
            match_readiness_service,
            notification_port,
        }
    }

    async fn get_match(&self, match_id: MatchId) -> Result<Match, MatchReadinessError> {
        match self
            .ports
            .transaction_manager()
            .execute(self.ports.match_repository().get_match(match_id))
            .await
        {
            Ok(Some(match_entry)) => Ok(match_entry),
            Ok(None) => Err(MatchReadinessError::MatchNotFound),
            Err(TransactionError(e)) => {
                tracing::error!("Database error while retrieving match {}: {}", match_id, e);
                Err(MatchReadinessError::Internal)
            }
        }
    }
}

#[derive(Debug)]
pub enum MatchReadinessError {
    MatchNotFound,
    Internal,
}

#[async_trait::async_trait]
impl<
    Ports: PortsCollection + Send + Sync + 'static,
    C: CreateGameFromMatchWorkflow + Send + Sync + 'static,
    RS: MatchReadinessService + Send + Sync + 'static,
    L: NotifyPlayerWorkflow + Send + Sync + 'static,
> MatchReadinessUseCase for MatchReadinessUseCaseImpl<Ports, C, RS, L>
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
        user_id: UserId,
    ) -> Result<(), MatchReadinessError> {
        let match_entry = self.get_match(match_id).await?;
        if match_entry.player1.user_id != user_id && match_entry.player2.user_id != user_id {
            tracing::error!(
                "Player {} is not a participant in match {}",
                user_id,
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
            .set_player_ready(match_id, user_id);
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
                    user_id: Some(user_id),
                },
            };

            self.notification_port
                .notify_players(
                    &[match_entry.player1.user_id, match_entry.player2.user_id],
                    &msg,
                )
                .await;
        }
        Ok(())
    }

    #[tracing::instrument(skip(self), ret, err(Debug))]
    async fn set_player_not_ready(
        &self,
        match_id: MatchId,
        user_id: UserId,
    ) -> Result<(), MatchReadinessError> {
        let match_entry = self.get_match(match_id).await?;
        if match_entry.player1.user_id != user_id && match_entry.player2.user_id != user_id {
            tracing::error!(
                "Player {} is not a participant in match {}",
                user_id,
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
            .set_player_not_ready(match_id, user_id);
        if did_remove {
            let msg = ListenerMessage::MatchEvent {
                match_id,
                event_type: ListenerMatchEventType::MatchReadinessChanged { user_id: None },
            };
            self.notification_port
                .notify_players(
                    &[match_entry.player1.user_id, match_entry.player2.user_id],
                    &msg,
                )
                .await;
        }
        Ok(())
    }
}
