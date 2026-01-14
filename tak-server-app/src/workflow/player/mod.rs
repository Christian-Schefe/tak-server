use crate::domain::{PlayerId, rating::PlayerRating, stats::PlayerStats};

pub mod get_rating;
pub mod get_stats;
pub mod notify_player;

#[derive(Clone, Debug)]
pub struct RatedPlayerView {
    pub player_id: PlayerId,
    pub rating: f64,
    pub max_rating: f64, //TODO: Remove and use stats to track max rating, this value should be for internal use only
    pub rated_games_played: u32, //TODO: Remove and use stats to track games played, this value should be for internal use only
    pub participation_rating: f64,
}

impl RatedPlayerView {
    pub fn from(player_rating: PlayerRating, participation_rating: f64) -> Self {
        Self {
            player_id: player_rating.player_id,
            rating: player_rating.rating,
            max_rating: player_rating.max_rating,
            rated_games_played: player_rating.rated_games_played,
            participation_rating,
        }
    }
}

pub struct PlayerStatsView {
    pub rated_games_played: u32,
    pub games_played: u32,
    pub games_won: u32,
    pub games_lost: u32,
    pub games_drawn: u32,
}

impl PlayerStatsView {
    pub fn from(stats: PlayerStats) -> Self {
        Self {
            rated_games_played: stats.rated_games_played,
            games_played: stats.games_played,
            games_won: stats.games_won,
            games_lost: stats.games_lost,
            games_drawn: stats.games_drawn,
        }
    }
}
