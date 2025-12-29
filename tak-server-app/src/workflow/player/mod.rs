use crate::domain::{PlayerId, rating::PlayerRating};

pub mod get_rating;
pub mod set_online;

#[derive(Clone, Debug)]
pub struct RatingView {
    pub player_id: PlayerId,
    pub rating: f64,
    pub max_rating: f64,
    pub rated_games_played: u32,
    pub is_unrated: bool,
    pub participation_rating: f64,
}

impl From<PlayerRating> for RatingView {
    fn from(player_rating: PlayerRating) -> Self {
        Self {
            player_id: player_rating.player_id,
            rating: player_rating.rating,
            max_rating: player_rating.max_rating,
            rated_games_played: player_rating.rated_games_played,
            is_unrated: player_rating.is_unrated,
            participation_rating: player_rating.participation_rating,
        }
    }
}
