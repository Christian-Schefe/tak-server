use chrono::{DateTime, Utc};
use tak_domain::{
    UserId,
    stats::{PlayerStats, RatingHistoryEntry, RatingHistoryRange},
};

use crate::errors::{AppResult, InternalError};

#[async_trait::async_trait]
pub trait StatsRepository<TrCtx> {
    async fn get_player_stats(
        &self,
        ctx: &TrCtx,
        user_id: UserId,
    ) -> AppResult<Option<PlayerStats>, (InternalError,)>;
    async fn set_player_stats(
        &self,
        ctx: &TrCtx,
        user_id: UserId,
        stats: PlayerStats,
    ) -> AppResult<(), (InternalError,)>;
    async fn remove_player_stats(
        &self,
        ctx: &TrCtx,
        user_id: UserId,
    ) -> AppResult<(), (InternalError,)>;
}

#[async_trait::async_trait]
pub trait RatingHistoryRepository<TrCtx> {
    async fn get_rating_history(
        &self,
        ctx: &TrCtx,
        user_id: UserId,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> AppResult<RatingHistoryRange, (InternalError,)>;
    async fn add_rating_history_entry(
        &self,
        ctx: &TrCtx,
        user_id: UserId,
        rating: RatingHistoryEntry,
    ) -> AppResult<(), (InternalError,)>;
}
