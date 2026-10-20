use std::sync::Arc;

use crate::{
    PortsCollection,
    ports::{
        puzzle::PuzzleRepository,
        transaction::{RepoError, TransactionManager},
    },
    workflow::puzzle::PuzzleView,
};
use tak_domain::PuzzleId;

#[async_trait::async_trait]
pub trait GetPuzzleUseCase {
    async fn get_puzzle(&self, id: PuzzleId) -> Result<PuzzleView, GetPuzzleError>;
    async fn select_random_puzzle(&self) -> Result<PuzzleId, ()>;
}

pub enum GetPuzzleError {
    NotFound,
    InternalError,
}

pub struct GetPuzzleUseCaseImpl<Ports: PortsCollection> {
    ports: Arc<Ports>,
}

impl<Ports: PortsCollection> GetPuzzleUseCaseImpl<Ports> {
    pub fn new(ports: Arc<Ports>) -> Self {
        Self { ports }
    }
}

#[async_trait::async_trait]
impl<Ports: PortsCollection + Send + Sync + 'static> GetPuzzleUseCase
    for GetPuzzleUseCaseImpl<Ports>
{
    async fn get_puzzle(&self, id: PuzzleId) -> Result<PuzzleView, GetPuzzleError> {
        let ctx = self.ports.transaction_manager().simple();
        match self.ports.puzzle_repository().get_puzzle(&ctx, id).await {
            Ok(Some(puzzle)) => Ok(PuzzleView::from(&puzzle)),
            Ok(None) => Err(GetPuzzleError::NotFound),
            Err(RepoError(e)) => {
                tracing::error!("Error retrieving puzzle {}: {}", id.0, e);
                Err(GetPuzzleError::InternalError)
            }
        }
    }

    async fn select_random_puzzle(&self) -> Result<PuzzleId, ()> {
        let ctx = self.ports.transaction_manager().simple();
        match self
            .ports
            .puzzle_repository()
            .select_random_puzzle(&ctx)
            .await
        {
            Ok(id) => Ok(id),
            Err(RepoError(e)) => {
                tracing::error!("Error selecting random puzzle: {}", e);
                Err(())
            }
        }
    }
}
