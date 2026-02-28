use std::sync::Arc;

use crate::{
    domain::{PuzzleId, RepoRetrieveError, puzzle::PuzzleRepository},
    workflow::puzzle::PuzzleView,
};

#[async_trait::async_trait]
pub trait GetPuzzleUseCase {
    async fn get_puzzle(&self, id: PuzzleId) -> Result<PuzzleView, GetPuzzleError>;
}

pub enum GetPuzzleError {
    NotFound,
    InternalError,
}

pub struct GetPuzzleUseCaseImpl<P: PuzzleRepository> {
    puzzle_repository: Arc<P>,
}

impl<P: PuzzleRepository> GetPuzzleUseCaseImpl<P> {
    pub fn new(puzzle_repository: Arc<P>) -> Self {
        Self { puzzle_repository }
    }
}

#[async_trait::async_trait]
impl<P: PuzzleRepository + Send + Sync + 'static> GetPuzzleUseCase for GetPuzzleUseCaseImpl<P> {
    async fn get_puzzle(&self, id: PuzzleId) -> Result<PuzzleView, GetPuzzleError> {
        let puzzle = self
            .puzzle_repository
            .get_puzzle(id)
            .await
            .map_err(|e| match e {
                RepoRetrieveError::NotFound => GetPuzzleError::NotFound,
                RepoRetrieveError::StorageError(msg) => {
                    log::error!("Error retrieving puzzle {}: {}", id.0, msg);
                    GetPuzzleError::InternalError
                }
            })?;
        Ok(PuzzleView::from(&puzzle))
    }
}
