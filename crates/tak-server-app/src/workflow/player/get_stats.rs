use std::sync::Arc;

use crate::{
    PortsCollection,
    ports::{
        rating::RatingRepository,
        stats::StatsRepository,
        transaction::{TransactionError, TransactionManager},
    },
    workflow::player::PlayerStatsView,
};
use tak_domain::{UserId, stats::PlayerStats};

#[async_trait::async_trait]
pub trait GetPlayerStatsUseCase {
    async fn get_stats(&self, user_id: UserId) -> Result<PlayerStatsView, GetStatsError>;
}

pub enum GetStatsError {
    Internal,
}

pub struct GetPlayerStatsUseCaseImpl<Ports: PortsCollection> {
    ports: Arc<Ports>,
}

impl<Ports: PortsCollection> GetPlayerStatsUseCaseImpl<Ports> {
    pub fn new(ports: Arc<Ports>) -> Self {
        Self { ports }
    }
}

#[async_trait::async_trait]
impl<Ports: PortsCollection + Send + Sync + 'static> GetPlayerStatsUseCase
    for GetPlayerStatsUseCaseImpl<Ports>
{
    async fn get_stats(&self, user_id: UserId) -> Result<PlayerStatsView, GetStatsError> {
        let rating_info = match self
            .ports
            .transaction_manager()
            .execute(self.ports.rating_repository().get_player_ranking(user_id))
            .await
        {
            Ok(Some(rating_info)) => Some(rating_info),
            Ok(None) => None,
            Err(TransactionError(e)) => {
                tracing::error!("Failed to retrieve player rating: {}", e);
                return Err(GetStatsError::Internal);
            }
        };
        let stats = match self
            .ports
            .transaction_manager()
            .execute(self.ports.stats_repository().get_player_stats(user_id))
            .await
        {
            Ok(Some(stats)) => stats,
            Ok(None) => PlayerStats {
                rated_games_played: 0,
                games_played: 0,
                games_won: 0,
                games_lost: 0,
                games_drawn: 0,
                win_streak: 0,
                longest_win_streak: 0,
            },
            Err(TransactionError(e)) => {
                tracing::error!("Failed to retrieve player stats: {}", e);
                return Err(GetStatsError::Internal);
            }
        };
        Ok(PlayerStatsView::from(rating_info, stats))
    }
}
