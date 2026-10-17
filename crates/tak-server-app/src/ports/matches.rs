use tak_domain::{MatchId, TournamentId, matches::Match};

use crate::ports::{RepoError, RepoRetrieveError};

#[async_trait::async_trait]
pub trait MatchRepository {
    async fn create_match(&self, new_match: Match) -> Result<MatchId, RepoError>;
    async fn get_match(&self, match_id: MatchId) -> Result<Match, RepoRetrieveError>;
    async fn update_match(&self, match_id: MatchId, updated_match: Match) -> Result<(), RepoError>;
    async fn get_matches_of_tournament(
        &self,
        tournament_id: TournamentId,
    ) -> Result<Vec<(MatchId, Match)>, RepoError>;
}
