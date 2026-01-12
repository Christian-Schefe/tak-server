use crate::domain::{PlayerId, rating::PlayerRating};

pub mod get_rating;
pub mod notify_player;

#[derive(Clone, Debug)]
pub struct RatedPlayerView {
    pub player_id: PlayerId,
    pub rating: f64,
    pub max_rating: f64,
    pub rated_games_played: u32,
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
