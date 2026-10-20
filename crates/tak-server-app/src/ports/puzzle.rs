use tak_domain::{PuzzleId, puzzle::Puzzle};

use crate::ports::transaction::RepoError;

#[async_trait::async_trait]
pub trait PuzzleRepository<RpCtx> {
    async fn get_puzzle(
        &self,
        ctx: &RpCtx,
        id: PuzzleId,
    ) -> Result<Option<Puzzle>, RepoError>;
    async fn select_random_puzzle(&self, ctx: &RpCtx) -> Result<PuzzleId, RepoError>;
}
