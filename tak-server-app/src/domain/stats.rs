use chrono::{DateTime, Utc};

use crate::domain::{PlayerId, RepoError, RepoRetrieveError};

#[async_trait::async_trait]
pub trait StatsRepository {
    async fn get_player_stats(&self, player_id: PlayerId)
    -> Result<PlayerStats, RepoRetrieveError>;
    async fn update_player_game(
        &self,
        player_id: PlayerId,
        result: GameOutcome,
        was_rated: bool,
    ) -> Result<(), RepoError>;
    async fn remove_player_stats(&self, player_id: PlayerId) -> Result<(), RepoError>;
}

#[async_trait::async_trait]
pub trait RatingHistoryRepository {
    async fn get_rating_history(
        &self,
        player_id: PlayerId,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Result<RatingHistoryRange, RepoError>;
    async fn add_rating_history_entry(
        &self,
        player_id: PlayerId,
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
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum GameOutcome {
    Win,
    Loss,
    Draw,
}
