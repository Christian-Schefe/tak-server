use chrono::{DateTime, Utc};
use tak_core::{TakGameResult, TakPlayer};

use crate::{
    GameId, MatchId, PaginatedResponse, Pagination, RepoError, RepoRetrieveError, SortOrder,
    UserId,
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

impl GameRecord {
    pub fn for_ongoing_game(metadata: GameMetadata, white: PlayerSnapshot, black: PlayerSnapshot) -> Self {
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
    UserId(UserId),
}

pub struct GameFinishedUpdate {
    pub result: TakGameResult,
    pub events: Vec<GameEvent>,
    pub rating_info: Option<GameRatingInfo>,
}

impl GameFinishedUpdate {
    pub fn from_game(game: FinishedGame, rating_info: Option<GameRatingInfo>) -> Self {
        Self {
            result: game.game.game_result().clone(),
            events: game.events.clone(),
            rating_info,
        }
    }
}

#[async_trait::async_trait]
pub trait GameRepository {
    async fn save_ongoing_game(&self, game: GameRecord) -> Result<GameId, RepoError>;
    async fn update_finished_game(
        &self,
        game_id: GameId,
        update: GameFinishedUpdate,
    ) -> Result<(), RepoRetrieveError>;
    async fn get_game_record(&self, game_id: GameId) -> Result<GameRecord, RepoRetrieveError>;
    async fn query_games(
        &self,
        query: GameQuery,
    ) -> Result<PaginatedResponse<(GameId, GameRecord)>, RepoError>;
    async fn get_games_of_match(
        &self,
        match_id: MatchId,
    ) -> Result<Vec<(GameId, GameRecord)>, RepoError>;
}
