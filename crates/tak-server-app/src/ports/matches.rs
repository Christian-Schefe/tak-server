use tak_domain::{MatchId, TournamentId, matches::Match};

use crate::ports::transaction::RepoError;

#[async_trait::async_trait]
pub trait MatchRepository<RpCtx> {
    async fn create_match(
        &self,
        ctx: &RpCtx,
        new_match: Match,
    ) -> Result<MatchId, RepoError>;
    async fn get_match(
        &self,
        ctx: &RpCtx,
        match_id: MatchId,
    ) -> Result<Option<Match>, RepoError>;
    async fn update_match(
        &self,
        ctx: &RpCtx,
        match_id: MatchId,
        updated_match: Match,
    ) -> Result<(), RepoError>;
    async fn get_matches_of_tournament(
        &self,
        ctx: &RpCtx,
        tournament_id: TournamentId,
    ) -> Result<Vec<(MatchId, Match)>, RepoError>;
}
