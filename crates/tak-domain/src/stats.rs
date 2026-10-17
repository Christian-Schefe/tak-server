use chrono::{DateTime, Utc};

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
