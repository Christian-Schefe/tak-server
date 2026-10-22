use std::sync::Arc;

use tak_domain::{GameId, ListenerId, game_record::GameRecordState, spectator::SpectatorService};

use crate::{
    PortsCollection,
    ports::{
        game::GameRepository,
        transaction::{RepoError, TransactionManager},
    },
};

#[async_trait::async_trait]
pub trait ObserveGameUseCase {
    async fn observe_game(
        &self,
        game_id: GameId,
        listener_id: ListenerId,
    ) -> Result<(), ObserveGameError>;
    fn unobserve_game(&self, game_id: GameId, listener_id: ListenerId);
}

pub enum ObserveGameError {
    GameNotFound,
    Internal,
}

pub struct ObserveGameUseCaseImpl<Ports: PortsCollection, S: SpectatorService> {
    ports: Arc<Ports>,
    spectator_service: Arc<S>,
}

impl<Ports: PortsCollection, S: SpectatorService> ObserveGameUseCaseImpl<Ports, S> {
    pub fn new(ports: Arc<Ports>, spectator_service: Arc<S>) -> Self {
        Self {
            ports,
            spectator_service,
        }
    }
}

#[async_trait::async_trait]
impl<Ports: PortsCollection + Send + Sync + 'static, S: SpectatorService + Send + Sync + 'static>
    ObserveGameUseCase for ObserveGameUseCaseImpl<Ports, S>
{
    async fn observe_game(
        &self,
        game_id: GameId,
        listener_id: ListenerId,
    ) -> Result<(), ObserveGameError> {
        let ctx = self.ports.transaction_manager().simple();
        match self.ports.game_repository().get_game(&ctx, game_id).await {
            Ok(None) => return Err(ObserveGameError::GameNotFound),
            Ok(Some(game)) => match game.state {
                GameRecordState::Ongoing => (),
                GameRecordState::Finished { .. } => return Err(ObserveGameError::GameNotFound),
            },
            Err(RepoError(e)) => {
                tracing::error!("Failed to retrieve game {} from repository: {}", game_id, e);
                return Err(ObserveGameError::Internal);
            }
        }
        self.spectator_service.observe_game(game_id, listener_id);
        Ok(())
    }

    fn unobserve_game(&self, game_id: GameId, listener_id: ListenerId) {
        self.spectator_service.unobserve_game(game_id, listener_id);
    }
}
