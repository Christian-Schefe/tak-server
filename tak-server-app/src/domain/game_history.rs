use chrono::{DateTime, Utc};
use tak_core::{TakGameResult, TakPlayer};

use crate::domain::{
    GameId, PaginatedResponse, Pagination, PlayerId, RepoError, RepoRetrieveError, RepoUpdateError,
    SortOrder,
    game::{FinishedGame, GameEvent, GameMetadata},
};

pub struct GameRecord {
    pub metadata: GameMetadata,
    pub white: PlayerSnapshot,
    pub black: PlayerSnapshot,
    pub rating_info: Option<GameRatingInfo>,
    pub result: Option<TakGameResult>,
    pub events: Vec<GameEvent>,
}

#[derive(Debug, Clone)]
pub struct PlayerSnapshot {
    pub username: Option<String>,
    pub rating: Option<f64>,
}

impl PlayerSnapshot {
    pub fn new(username: Option<String>, rating: Option<f64>) -> Self {
        Self { username, rating }
    }
}

#[derive(Debug, Clone)]
pub struct GameRatingInfo {
    pub rating_change_white: f64,
    pub rating_change_black: f64,
}

#[derive(Debug, Clone, Default)]
pub struct GameQuery {
    pub id_selector: Option<GameIdSelector>,
    pub date_selector: Option<DateSelector>,
    pub player_filters: Vec<(GamePlayerFilter, Option<TakPlayer>)>,
    pub game_results: Option<Vec<TakGameResult>>,
    pub half_komi: Option<usize>,
    pub board_size: Option<usize>,
    pub is_rated: Option<bool>,
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
    PlayerId(PlayerId),
}

pub struct GameFinishedUpdate {
    pub result: TakGameResult,
    pub events: Vec<GameEvent>,
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
        metadata: GameMetadata,
        white: PlayerSnapshot,
        black: PlayerSnapshot,
    ) -> GameRecord;
    fn get_finished_game_record_update(
        &self,
        game: FinishedGame,
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
        metadata: GameMetadata,
        white: PlayerSnapshot,
        black: PlayerSnapshot,
    ) -> GameRecord {
        GameRecord {
            metadata,
            white,
            black,
            rating_info: None,
            result: None,
            events: Vec::new(),
        }
    }

    fn get_finished_game_record_update(
        &self,
        game: FinishedGame,
        rating_info: Option<GameRatingInfo>,
    ) -> GameFinishedUpdate {
        GameFinishedUpdate {
            result: game.game.game_result().clone(),
            events: game.events.clone(),
            rating_info,
        }
    }
}
