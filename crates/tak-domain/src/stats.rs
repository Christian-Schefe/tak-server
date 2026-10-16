use chrono::{DateTime, Utc};

use crate::{RepoError, RepoRetrieveError, UserId};

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

pub struct RatingHistoryRange {
    pub entries: Vec<RatingHistoryEntry>,
    pub first_entry_before_range: Option<RatingHistoryEntry>,
}

pub struct RatingHistoryEntry {
    pub timestamp: DateTime<Utc>,
    pub rating: f64,
}

impl RatingHistoryEntry {
    pub fn new(timestamp: DateTime<Utc>, rating: f64) -> Self {
        Self { timestamp, rating }
    }
}

#[derive(Clone, Debug)]
pub struct PlayerStats {
    pub rated_games_played: u32,
    pub games_played: u32,
    pub games_won: u32,
    pub games_lost: u32,
    pub games_drawn: u32,
    pub win_streak: u32,
    pub longest_win_streak: u32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum GameOutcome {
    Win,
    Loss,
    Draw,
}
