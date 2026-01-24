use std::time::Duration;

use chrono::{DateTime, Utc};
use tak_core::{TakAction, TakBaseGameSettings, TakGameResult, TakRealtimeGameSettings};

use crate::domain::{
    GameId, PaginatedResponse, Pagination, PlayerId, RepoError, RepoRetrieveError, RepoUpdateError,
    SortOrder,
    game::{FinishedGame, GameEvent, GameEventType},
};

pub struct GameRecord {
    pub date: DateTime<Utc>,
    pub white: PlayerSnapshot,
    pub black: PlayerSnapshot,
    pub rating_info: Option<GameRatingInfo>,
    pub settings: GameSettings,
    pub is_rated: bool,
    pub result: Option<TakGameResult>,
    pub events: Vec<GameEvent>,
}

pub enum GameSettings {
    Realtime(TakRealtimeGameSettings),
    Async(TakBaseGameSettings),
}

impl GameRecord {
    pub fn reconstruct_action_history(&self) -> Vec<TakAction> {
        let mut actions = Vec::new();
        for event in &self.events {
            if let GameEventType::Action { action, .. } = &event.event_type {
                actions.push(action.clone());
            } else if let GameEventType::ActionUndone = &event.event_type {
                actions.pop();
            }
        }
        actions
    }

    pub fn reconstruct_time_remaining(&self) -> (Duration, Duration) {
        let GameSettings::Realtime(settings) = &self.settings else {
            return (Duration::ZERO, Duration::ZERO);
        };
        let mut white_remaining = settings.time_control.contingent;
        let mut black_remaining = settings.time_control.contingent;

        for event in &self.events {
            match &event.event_type {
                GameEventType::Action {
                    white_remaining: w,
                    black_remaining: b,
                    ..
                } => {
                    white_remaining = *w;
                    black_remaining = *b;
                }
                _ => {}
            }
        }

        (white_remaining, black_remaining)
    }
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
    pub game_results: Option<Vec<TakGameResult>>,
    pub half_komi: Option<usize>,
    pub board_size: Option<usize>,
    pub is_rated: Option<bool>,
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
        date: DateTime<Utc>,
        white: PlayerSnapshot,
        black: PlayerSnapshot,
        settings: GameSettings,
        is_rated: bool,
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
        date: DateTime<Utc>,
        white: PlayerSnapshot,
        black: PlayerSnapshot,
        settings: GameSettings,
        is_rated: bool,
    ) -> GameRecord {
        GameRecord {
            date,
            white,
            black,
            rating_info: None,
            settings,
            is_rated,
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
