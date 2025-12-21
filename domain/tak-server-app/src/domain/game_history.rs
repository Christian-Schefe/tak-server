use std::sync::Arc;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use tak_core::{TakActionRecord, TakGameSettings, TakGameState};

use crate::domain::{FinishedGameId, GameId, GameType, PlayerId, game::FinishedGame};

pub struct GameRecord {
    pub date: DateTime<Utc>,
    pub white: PlayerId,
    pub black: PlayerId,
    pub rating_info: Option<GameRatingInfo>,
    pub settings: TakGameSettings,
    pub game_type: GameType,
    pub result: TakGameState,
    pub moves: Vec<TakActionRecord>,
}

#[derive(Clone, Debug)]
pub struct GameRatingInfo {
    pub rating_white: f64,
    pub rating_black: f64,
    pub rating_change: Option<(f64, f64)>,
}

pub trait GameRepository {
    fn save_ongoing_game(&self, game: GameRecord) -> FinishedGameId;
    fn update_finished_game(&self, game_id: FinishedGameId, game: GameRecord);
    fn get_game_record(&self, game_id: FinishedGameId) -> Option<GameRecord>;
}

pub trait GameHistoryService {
    fn save_ongoing_game_id(&self, game_id: GameId, finished_game_id: FinishedGameId);
    fn remove_ongoing_game_id(&self, game_id: GameId) -> Option<FinishedGameId>;
    fn get_ongoing_game_record(
        &self,
        white: PlayerId,
        black: PlayerId,
        settings: TakGameSettings,
        game_type: GameType,
    ) -> GameRecord;
    fn get_finished_game_record(
        &self,
        game: FinishedGame,
        rating_info: Option<GameRatingInfo>,
    ) -> GameRecord;
}

pub struct GameHistoryServiceImpl {
    game_ids: Arc<DashMap<GameId, FinishedGameId>>,
}

impl GameHistoryServiceImpl {
    pub fn new() -> Self {
        Self {
            game_ids: Arc::new(DashMap::new()),
        }
    }
}

impl GameHistoryService for GameHistoryServiceImpl {
    fn save_ongoing_game_id(&self, game_id: GameId, finished_game_id: FinishedGameId) {
        self.game_ids.insert(game_id, finished_game_id);
    }

    fn remove_ongoing_game_id(&self, game_id: GameId) -> Option<FinishedGameId> {
        self.game_ids.remove(&game_id).map(|(_, v)| v)
    }

    fn get_ongoing_game_record(
        &self,
        white: PlayerId,
        black: PlayerId,
        settings: TakGameSettings,
        game_type: GameType,
    ) -> GameRecord {
        GameRecord {
            date: Utc::now(),
            white,
            black,
            rating_info: None,
            settings,
            game_type,
            result: TakGameState::Ongoing,
            moves: Vec::new(),
        }
    }

    fn get_finished_game_record(
        &self,
        game: FinishedGame,
        rating_info: Option<GameRatingInfo>,
    ) -> GameRecord {
        GameRecord {
            date: Utc::now(),
            white: game.white,
            black: game.black,
            rating_info,
            settings: game.settings,
            game_type: game.game_type,
            result: game.result,
            moves: game.moves,
        }
    }
}
