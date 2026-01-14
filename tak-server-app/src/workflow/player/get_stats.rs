use std::sync::Arc;

use crate::{
    domain::{
        PlayerId, RepoRetrieveError,
        stats::{PlayerStats, StatsRepository},
    },
    workflow::player::PlayerStatsView,
};

#[async_trait::async_trait]
pub trait GetPlayerStatsUseCase {
    async fn get_stats(&self, player_id: PlayerId) -> Result<PlayerStatsView, GetStatsError>;
}

pub enum GetStatsError {
    Internal,
}

pub struct GetPlayerStatsUseCaseImpl<S: StatsRepository> {
    stats_repo: Arc<S>,
}

impl<S: StatsRepository> GetPlayerStatsUseCaseImpl<S> {
    pub fn new(stats_repo: Arc<S>) -> Self {
        Self { stats_repo }
    }
}

#[async_trait::async_trait]
impl<S: StatsRepository + Send + Sync + 'static> GetPlayerStatsUseCase
    for GetPlayerStatsUseCaseImpl<S>
{
    async fn get_stats(&self, player_id: PlayerId) -> Result<PlayerStatsView, GetStatsError> {
        let stats = match self.stats_repo.get_player_stats(player_id).await {
            Ok(stats) => stats,
            Err(RepoRetrieveError::NotFound) => PlayerStats {
                rated_games_played: 0,
                games_played: 0,
                games_won: 0,
                games_lost: 0,
                games_drawn: 0,
            },
            Err(RepoRetrieveError::StorageError(e)) => {
                log::error!("Failed to retrieve player stats: {}", e);
                return Err(GetStatsError::Internal);
            }
        };
        Ok(PlayerStatsView::from(stats))
    }
}
