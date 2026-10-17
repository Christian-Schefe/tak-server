use tak_core::TakGameResult;

use crate::game::{GameEvent, GameMetadata};

pub struct GameRecord {
    pub metadata: GameMetadata,
    pub white: PlayerSnapshot,
    pub black: PlayerSnapshot,
    pub rating_info: Option<GameRatingInfo>,
    pub result: Option<TakGameResult>,
    pub events: Vec<GameEvent>,
}

impl GameRecord {
    pub fn for_ongoing_game(
        metadata: GameMetadata,
        white: PlayerSnapshot,
        black: PlayerSnapshot,
    ) -> Self {
        Self {
            metadata,
            white,
            black,
            rating_info: None,
            result: None,
            events: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlayerSnapshot {
    pub username: String,
    pub rating: Option<f64>,
}

impl PlayerSnapshot {
    pub fn new(username: String, rating: Option<f64>) -> Self {
        Self { username, rating }
    }
}

#[derive(Debug, Clone)]
pub struct GameRatingInfo {
    pub rating_change_white: f64,
    pub rating_change_black: f64,
}
