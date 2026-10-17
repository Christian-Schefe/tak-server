use tak_domain::{PuzzleId, puzzle::Puzzle};

use crate::ports::{RepoError, RepoRetrieveError};

#[async_trait::async_trait]
pub trait PuzzleRepository {
    async fn get_puzzle(&self, id: PuzzleId) -> Result<Puzzle, RepoRetrieveError>;
    async fn select_random_puzzle(&self) -> Result<PuzzleId, RepoError>;
}
