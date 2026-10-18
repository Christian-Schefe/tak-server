use tak_domain::{MatchId, TournamentId, matches::Match};

use crate::ports::transaction::{TransactionError, TransactionItem};

pub trait MatchRepository<TrCtx> {
    fn create_match(
        &self,
        new_match: Match,
    ) -> TransactionItem<TrCtx, Result<MatchId, TransactionError>>;
    fn get_match(
        &self,
        match_id: MatchId,
    ) -> TransactionItem<TrCtx, Result<Option<Match>, TransactionError>>;
    fn update_match(
        &self,
        match_id: MatchId,
        updated_match: Match,
    ) -> TransactionItem<TrCtx, Result<(), TransactionError>>;
    fn get_matches_of_tournament(
        &self,
        tournament_id: TournamentId,
    ) -> TransactionItem<TrCtx, Result<Vec<(MatchId, Match)>, TransactionError>>;
}
