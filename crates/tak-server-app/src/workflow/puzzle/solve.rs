use std::sync::Arc;

use tak_core::TakAction;

use crate::domain::{
    PuzzleId, RepoRetrieveError,
    puzzle::{PuzzleRepository, PuzzleResponse},
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

pub struct SolvePuzzleUseCaseImpl<P: PuzzleRepository> {
    puzzle_repository: Arc<P>,
}

impl<P: PuzzleRepository> SolvePuzzleUseCaseImpl<P> {
    pub fn new(puzzle_repository: Arc<P>) -> Self {
        Self { puzzle_repository }
    }
}

#[async_trait::async_trait]
impl<P: PuzzleRepository + Send + Sync + 'static> SolvePuzzleUseCase for SolvePuzzleUseCaseImpl<P> {
    async fn attempt_solve_puzzle(
        &self,
        id: PuzzleId,
        actions: Vec<TakAction>,
    ) -> Result<PuzzleResponse, SolvePuzzleError> {
        let puzzle = self
            .puzzle_repository
            .get_puzzle(id)
            .await
            .map_err(|e| match e {
                RepoRetrieveError::NotFound => SolvePuzzleError::NotFound,
                RepoRetrieveError::StorageError(_) => SolvePuzzleError::InternalError,
            })?;
        Ok(puzzle
            .do_response(&actions)
            .ok_or_else(|| SolvePuzzleError::InvalidInput("Invalid action sequence".to_string()))?)
    }
}
