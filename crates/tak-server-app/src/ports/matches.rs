use tak_domain::{MatchId, TournamentId, matches::Match};

use crate::ports::{RepoError, RepoRetrieveError};

#[async_trait::async_trait]
pub trait MatchRepository<TrCtx> {
    async fn create_match(&self, ctx: &TrCtx, new_match: Match) -> Result<MatchId, RepoError>;
    async fn get_match(&self, ctx: &TrCtx, match_id: MatchId) -> Result<Match, RepoRetrieveError>;
    async fn update_match(
        &self,
        ctx: &TrCtx,
        match_id: MatchId,
        updated_match: Match,
    ) -> Result<(), RepoError>;
    async fn get_matches_of_tournament(
        &self,
        ctx: &TrCtx,
        tournament_id: TournamentId,
    ) -> Result<Vec<(MatchId, Match)>, RepoError>;
}
