use tak_core::{TakActionRecord, TakGameResult, TakTimeInfo};

use crate::game::GameMetadata;

#[derive(Debug, Clone)]
pub struct GameRatingInfo {
    pub rating_change_white: f64,
    pub rating_change_black: f64,
}

#[derive(Debug, Clone)]
pub struct GameRecord {
    pub metadata: GameMetadata,
    pub state: GameRecordState,
}

impl GameRecord {
    pub fn new_ongoing(metadata: GameMetadata) -> Self {
        Self {
            metadata,
            state: GameRecordState::Ongoing,
        }
    }
}

#[derive(Debug, Clone)]
pub enum GameRecordState {
    Ongoing,
    Finished {
        rating_info: Option<GameRatingInfo>,
        result: TakGameResult,
        time_info: TakTimeInfo,
        history: Vec<TakActionRecord>,
    },
}
