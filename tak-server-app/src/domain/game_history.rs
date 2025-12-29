use std::{sync::Arc, time::Duration};

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use tak_core::{TakActionRecord, TakGameSettings, TakGameState};

use crate::domain::{
    FinishedGameId, GameId, GameType, PaginatedResponse, Pagination, PlayerId, RepoError,
    RepoRetrieveError, RepoUpdateError, SortOrder, game::Game,
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
    Range(FinishedGameId, FinishedGameId),
    AndBefore(FinishedGameId),
    AndAfter(FinishedGameId),
    List(Vec<FinishedGameId>),
}

#[derive(Debug, Clone)]
pub enum GamePlayerFilter {
    Contains(String),
    Equals(String),
}

pub struct GameFinishedUpdate {
    pub result: TakGameState,
    pub moves: Vec<TakActionRecord>,
    pub player_white: PlayerSnapshot,
    pub player_black: PlayerSnapshot,
    pub rating_info: Option<GameRatingInfo>,
}

#[async_trait::async_trait]
pub trait GameRepository {
    async fn save_ongoing_game(&self, game: GameRecord) -> Result<FinishedGameId, RepoError>;
    async fn update_finished_game(
        &self,
        game_id: FinishedGameId,
        update: GameFinishedUpdate,
    ) -> Result<(), RepoUpdateError>;
    async fn get_game_record(
        &self,
        game_id: FinishedGameId,
    ) -> Result<GameRecord, RepoRetrieveError>;
    async fn query_games(
        &self,
        query: GameQuery,
    ) -> Result<PaginatedResponse<(FinishedGameId, GameRecord)>, RepoError>;
}

pub trait GameHistoryService {
    fn save_ongoing_game_id(&self, game_id: GameId, finished_game_id: FinishedGameId);
    fn remove_ongoing_game_id(&self, game_id: GameId) -> Option<FinishedGameId>;
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
        white: PlayerSnapshot,
        black: PlayerSnapshot,
        rating_info: Option<GameRatingInfo>,
    ) -> GameFinishedUpdate;
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
        white: PlayerSnapshot,
        black: PlayerSnapshot,
        rating_info: Option<GameRatingInfo>,
    ) -> GameFinishedUpdate {
        GameFinishedUpdate {
            result: game.game.game_state(),
            moves: game.game.action_history().clone(),
            player_white: white,
            player_black: black,
            rating_info,
        }
    }
}
