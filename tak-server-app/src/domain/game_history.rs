use std::time::Duration;

use chrono::{DateTime, Utc};
use tak_core::{TakActionRecord, TakGameSettings, TakGameState};

use crate::domain::{
    GameId, GameType, PaginatedResponse, Pagination, PlayerId, RepoError, RepoRetrieveError,
    RepoUpdateError, SortOrder, game::Game,
};

pub struct GameRecord {
    pub date: DateTime<Utc>,
    pub white: PlayerSnapshot,
    pub black: PlayerSnapshot,
    pub rating_info: Option<GameRatingInfo>,
    pub settings: TakGameSettings,
    pub game_type: GameType,
    pub result: TakGameState,
    pub moves: Vec<TakActionRecord>,
}

#[derive(Debug, Clone)]
pub struct PlayerSnapshot {
    pub player_id: PlayerId,
    pub username: Option<String>,
    pub rating: Option<f64>,
}

impl PlayerSnapshot {
    pub fn new(player_id: PlayerId, username: Option<String>, rating: Option<f64>) -> Self {
        Self {
            player_id,
            username,
            rating,
        }
    }
}

pub struct GameRatingInfo {
    pub rating_change_white: f64,
    pub rating_change_black: f64,
}

#[derive(Debug, Clone, Default)]
pub struct GameQuery {
    pub id_selector: Option<GameIdSelector>,
    pub date_selector: Option<DateSelector>,
    pub player_white: Option<GamePlayerFilter>,
    pub player_black: Option<GamePlayerFilter>,
    pub game_states: Option<Vec<TakGameState>>,
    pub half_komi: Option<usize>,
    pub board_size: Option<usize>,
    pub game_type: Option<GameType>,
    pub clock_contingent: Option<Duration>,
    pub clock_increment: Option<Duration>,
    pub clock_extra_trigger: Option<usize>,
    pub clock_extra_time: Option<Duration>,
    pub pagination: Pagination,
    pub sort: Option<(SortOrder, GameSortBy)>,
}

#[derive(Debug, Clone)]
pub enum GameSortBy {
    Date,
    GameId,
}

#[derive(Debug, Clone)]
pub enum DateSelector {
    Range(DateTime<Utc>, DateTime<Utc>),
    Before(DateTime<Utc>),
    After(DateTime<Utc>),
}

#[derive(Debug, Clone)]
pub enum GameIdSelector {
    Range(GameId, GameId),
    AndBefore(GameId),
    AndAfter(GameId),
    List(Vec<GameId>),
}

#[derive(Debug, Clone)]
pub enum GamePlayerFilter {
    Contains(String),
    Equals(String),
}

pub struct GameFinishedUpdate {
    pub result: TakGameState,
    pub moves: Vec<TakActionRecord>,
    pub rating_info: Option<GameRatingInfo>,
}

#[async_trait::async_trait]
pub trait GameRepository {
    async fn save_ongoing_game(&self, game: GameRecord) -> Result<GameId, RepoError>;
    async fn update_finished_game(
        &self,
        game_id: GameId,
        update: GameFinishedUpdate,
    ) -> Result<(), RepoUpdateError>;
    async fn get_game_record(&self, game_id: GameId) -> Result<GameRecord, RepoRetrieveError>;
    async fn query_games(
        &self,
        query: GameQuery,
    ) -> Result<PaginatedResponse<(GameId, GameRecord)>, RepoError>;
}

pub trait GameHistoryService {
    fn get_ongoing_game_record(
        &self,
        date: DateTime<Utc>,
        white_id: PlayerId,
        black_id: PlayerId,
        settings: TakGameSettings,
        game_type: GameType,
    ) -> GameRecord;
    fn get_finished_game_record_update(
        &self,
        game: Game,
        rating_info: Option<GameRatingInfo>,
    ) -> GameFinishedUpdate;
}

pub struct GameHistoryServiceImpl;

impl GameHistoryServiceImpl {
    pub fn new() -> Self {
        Self
    }
}

impl GameHistoryService for GameHistoryServiceImpl {
    fn get_ongoing_game_record(
        &self,
        date: DateTime<Utc>,
        white_id: PlayerId,
        black_id: PlayerId,
        settings: TakGameSettings,
        game_type: GameType,
    ) -> GameRecord {
        GameRecord {
            date,
            white: PlayerSnapshot::new(white_id, None, None),
            black: PlayerSnapshot::new(black_id, None, None),
            rating_info: None,
            settings,
            game_type,
            result: TakGameState::Ongoing,
            moves: Vec::new(),
        }
    }

    fn get_finished_game_record_update(
        &self,
        game: Game,
        rating_info: Option<GameRatingInfo>,
    ) -> GameFinishedUpdate {
        GameFinishedUpdate {
            result: game.game.game_state(),
            moves: game.game.action_history().clone(),
            rating_info,
        }
    }
}
