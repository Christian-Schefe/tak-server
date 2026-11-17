use std::sync::Arc;

use dashmap::DashMap;
use tak_domain::{MatchId, TournamentId, UserId, matches::Match};

use crate::errors::{AppResult, InternalError, MatchNotFound};

#[async_trait::async_trait]
pub trait MatchRepository<RpCtx> {
    async fn create_match(
        &self,
        ctx: &RpCtx,
        new_match: Match,
    ) -> AppResult<MatchId, (InternalError,)>;
    async fn get_match(
        &self,
        ctx: &RpCtx,
        match_id: MatchId,
    ) -> AppResult<Option<Match>, (InternalError,)>;
    async fn update_match(
        &self,
        ctx: &RpCtx,
        match_id: MatchId,
        updated_match: Match,
    ) -> AppResult<(), (MatchNotFound, InternalError)>;
    async fn get_matches_of_tournament(
        &self,
        ctx: &RpCtx,
        tournament_id: TournamentId,
    ) -> AppResult<Vec<(MatchId, Match)>, (InternalError,)>;
}

pub trait MatchReadinessService {
    fn get_readiness_status(&self, match_id: MatchId) -> Option<UserId>;
    fn set_player_ready(&self, match_id: MatchId, player: UserId) -> bool;
    fn set_player_not_ready(&self, match_id: MatchId, player: UserId) -> bool;
    fn set_player_not_ready_everywhere(&self, player: UserId) -> Vec<MatchId>;
}

pub struct MatchReadinessServiceImpl {
    match_readiness: Arc<DashMap<MatchId, UserId>>,
}

impl MatchReadinessServiceImpl {
    pub fn new() -> Self {
        Self {
            match_readiness: Arc::new(DashMap::new()),
        }
    }
}

impl MatchReadinessService for MatchReadinessServiceImpl {
    fn get_readiness_status(&self, match_id: MatchId) -> Option<UserId> {
        self.match_readiness
            .get(&match_id)
            .map(|entry| *entry.value())
    }

    fn set_player_ready(&self, match_id: MatchId, player: UserId) -> bool {
        let request = self.match_readiness.get(&match_id);
        if let Some(already_ready) = &request {
            if *already_ready.value() == player {
                false
            } else {
                drop(request);
                self.match_readiness.remove(&match_id);
                true
            }
        } else {
            drop(request);
            self.match_readiness.insert(match_id, player);
            false
        }
    }

    fn set_player_not_ready(&self, match_id: MatchId, player: UserId) -> bool {
        let request = self.match_readiness.get(&match_id);
        if let Some(already_ready) = &request {
            if *already_ready.value() == player {
                drop(request);
                self.match_readiness.remove(&match_id);
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    fn set_player_not_ready_everywhere(&self, player: UserId) -> Vec<MatchId> {
        let mut match_ids = Vec::new();
        self.match_readiness.retain(|match_id, ready_player| {
            if *ready_player == player {
                match_ids.push(*match_id);
                false
            } else {
                true
            }
        });
        match_ids
    }
}
