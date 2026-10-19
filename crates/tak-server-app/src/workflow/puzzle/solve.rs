use std::sync::Arc;

use tak_core::TakAction;

use tak_domain::{PuzzleId, puzzle::PuzzleResponse};

use crate::{
    PortsCollection,
    ports::{
        puzzle::PuzzleRepository,
        transaction::{TransactionError, TransactionManager},
    },
};

#[async_trait::async_trait]
pub trait SolvePuzzleUseCase {
    async fn attempt_solve_puzzle(
        &self,
        id: PuzzleId,
        actions: Vec<TakAction>,
    ) -> Result<PuzzleResponse, SolvePuzzleError>;
}

pub enum SolvePuzzleError {
    NotFound,
    InternalError,
    InvalidInput(String),
}

pub struct SolvePuzzleUseCaseImpl<Ports: PortsCollection> {
    ports: Arc<Ports>,
}

impl<Ports: PortsCollection> SolvePuzzleUseCaseImpl<Ports> {
    pub fn new(ports: Arc<Ports>) -> Self {
        Self { ports }
    }
}

#[async_trait::async_trait]
impl<Ports: PortsCollection + Send + Sync + 'static> SolvePuzzleUseCase
    for SolvePuzzleUseCaseImpl<Ports>
{
    async fn attempt_solve_puzzle(
        &self,
        id: PuzzleId,
        actions: Vec<TakAction>,
    ) -> Result<PuzzleResponse, SolvePuzzleError> {
        match self
            .ports
            .transaction_manager()
            .execute(self.ports.puzzle_repository().get_puzzle(id))
            .await
        {
            Ok(Some(puzzle)) => Ok(puzzle.do_response(&actions).ok_or_else(|| {
                SolvePuzzleError::InvalidInput("Invalid action sequence".to_string())
            })?),
            Ok(None) => Err(SolvePuzzleError::NotFound),
            Err(TransactionError(e)) => {
                tracing::error!("Error retrieving puzzle {}: {}", id.0, e);
                Err(SolvePuzzleError::InternalError)
            }
        }
    }
}
