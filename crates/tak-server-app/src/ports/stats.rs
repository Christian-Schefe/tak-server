use chrono::{DateTime, Utc};
use tak_domain::{
    UserId,
    stats::{PlayerStats, RatingHistoryEntry, RatingHistoryRange},
};

use crate::ports::transaction::{TransactionError, TransactionItem};

pub trait StatsRepository<TrCtx> {
    fn get_player_stats(
        &self,
        user_id: UserId,
    ) -> TransactionItem<TrCtx, Result<Option<PlayerStats>, TransactionError>>;
    fn set_player_stats(
        &self,
        user_id: UserId,
        stats: PlayerStats,
    ) -> TransactionItem<TrCtx, Result<(), TransactionError>>;
    fn remove_player_stats(
        &self,
        user_id: UserId,
    ) -> TransactionItem<TrCtx, Result<(), TransactionError>>;
}

pub trait RatingHistoryRepository<TrCtx> {
    fn get_rating_history(
        &self,
        user_id: UserId,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> TransactionItem<TrCtx, Result<RatingHistoryRange, TransactionError>>;
    fn add_rating_history_entry(
        &self,
        user_id: UserId,
        rating: RatingHistoryEntry,
    ) -> TransactionItem<TrCtx, Result<(), TransactionError>>;
}
