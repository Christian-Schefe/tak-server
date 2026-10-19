use tak_domain::{PuzzleId, puzzle::Puzzle};

use crate::ports::transaction::{TransactionError, TransactionItem};

pub trait PuzzleRepository<TrCtx> {
    fn get_puzzle(
        &self,
        id: PuzzleId,
    ) -> TransactionItem<TrCtx, Result<Option<Puzzle>, TransactionError>>;
    fn select_random_puzzle(&self) -> TransactionItem<TrCtx, Result<PuzzleId, TransactionError>>;
}
