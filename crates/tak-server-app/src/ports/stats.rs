use chrono::{DateTime, Utc};
use tak_domain::{
    UserId,
    stats::{PlayerStats, RatingHistoryEntry, RatingHistoryRange},
};

use crate::ports::{RepoError, RepoRetrieveError};

#[async_trait::async_trait]
pub trait StatsRepository {
    async fn get_player_stats(&self, user_id: UserId) -> Result<PlayerStats, RepoRetrieveError>;
    async fn update_player_game(
        &self,
        user_id: UserId,
        calc_fn: impl FnOnce(Option<PlayerStats>) -> PlayerStats + Send + 'static,
    ) -> Result<(), RepoError>;
    async fn remove_player_stats(&self, user_id: UserId) -> Result<(), RepoError>;
}

#[async_trait::async_trait]
pub trait RatingHistoryRepository {
    async fn get_rating_history(
        &self,
        user_id: UserId,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Result<RatingHistoryRange, RepoError>;
    async fn add_rating_history_entry(
        &self,
        user_id: UserId,
        rating: RatingHistoryEntry,
    ) -> Result<(), RepoError>;
}
