use std::sync::Arc;

use crate::{
    domain::{
        PlayerId, RepoRetrieveError,
        rating::RatingRepository,
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

pub struct GetPlayerStatsUseCaseImpl<S: StatsRepository, R: RatingRepository> {
    stats_repo: Arc<S>,
    rating_repo: Arc<R>,
}

impl<S: StatsRepository, R: RatingRepository> GetPlayerStatsUseCaseImpl<S, R> {
    pub fn new(stats_repo: Arc<S>, rating_repo: Arc<R>) -> Self {
        Self {
            stats_repo,
            rating_repo,
        }
    }
}

#[async_trait::async_trait]
impl<S: StatsRepository + Send + Sync + 'static, R: RatingRepository + Send + Sync + 'static>
    GetPlayerStatsUseCase for GetPlayerStatsUseCaseImpl<S, R>
{
    async fn get_stats(&self, player_id: PlayerId) -> Result<PlayerStatsView, GetStatsError> {
        let rating_info = match self.rating_repo.get_player_ranking(player_id).await {
            Ok(rating_info) => Some(rating_info),
            Err(RepoRetrieveError::NotFound) => None,
            Err(RepoRetrieveError::StorageError(e)) => {
                tracing::error!("Failed to retrieve player rating: {}", e);
                return Err(GetStatsError::Internal);
            }
        };
        let stats = match self.stats_repo.get_player_stats(player_id).await {
            Ok(stats) => stats,
            Err(RepoRetrieveError::NotFound) => PlayerStats {
                rated_games_played: 0,
                games_played: 0,
                games_won: 0,
                games_lost: 0,
                games_drawn: 0,
                win_streak: 0,
                longest_win_streak: 0,
            },
            Err(RepoRetrieveError::StorageError(e)) => {
                tracing::error!("Failed to retrieve player stats: {}", e);
                return Err(GetStatsError::Internal);
            }
        };
        Ok(PlayerStatsView::from(rating_info, stats))
    }
}
