use chrono::{DateTime, Utc};

use crate::domain::{rating::PlayerRating, stats::PlayerStats};

pub mod get_rating;
pub mod get_stats;
pub mod notify_player;

pub struct PlayerStatsView {
    pub ranking: Option<PlayerRankingView>,
    pub rated_games_played: u32,
    pub games_played: u32,
    pub games_won: u32,
    pub games_lost: u32,
    pub games_drawn: u32,
    pub win_streak: u32,
    pub longest_win_streak: u32,
}

pub struct PlayerRankingView {
    pub rating: f64,
    pub max_rating: f64,
    pub ranking: u32,
}

impl PlayerStatsView {
    pub fn from(rating_info: Option<(u32, PlayerRating)>, stats: PlayerStats) -> Self {
        Self {
            ranking: rating_info.map(|(rank, rating)| PlayerRankingView {
                rating: rating.rating,
                max_rating: rating.max_rating,
                ranking: rank,
            }),
            rated_games_played: stats.rated_games_played,
            games_played: stats.games_played,
            games_won: stats.games_won,
            games_lost: stats.games_lost,
            games_drawn: stats.games_drawn,
            win_streak: stats.win_streak,
            longest_win_streak: stats.longest_win_streak,
        }
    }
}

pub struct RatingHistoryEntryView {
    pub timestamp: DateTime<Utc>,
    pub rating: f64,
}

pub struct RatingHistoryRangeView {
    pub entries: Vec<RatingHistoryEntryView>,
    pub first_entry_before_range: Option<RatingHistoryEntryView>,
}
