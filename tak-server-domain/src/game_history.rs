use std::sync::Arc;

use chrono::{DateTime, Utc};
use tak_core::TakGameState;

use crate::{
    ServiceResult,
    game::{ArcGameRepository, GameId, GameRecord, GameType},
    player::PlayerUsername,
};

#[derive(Debug, Clone, Default)]
pub struct GameFilter {
    pub id_selector: Option<GameIdSelector>,
    pub date_selector: Option<DateSelector>,
    pub player_white: Option<PlayerUsername>,
    pub player_black: Option<PlayerUsername>,
    pub game_states: Option<Vec<TakGameState>>,
    pub half_komi: Option<usize>,
    pub board_size: Option<usize>,
    pub game_type: Option<GameType>,
    pub pagination: GamePagination,
}

pub struct GameFilterResult {
    pub total_count: usize,
    pub games: Vec<(GameId, GameRecord)>,
}

#[derive(Debug, Clone, Default)]
pub struct GamePagination {
    pub offset: Option<usize>,
    pub limit: Option<usize>,
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

#[async_trait::async_trait]
pub trait GameHistoryService {
    async fn get_game_record(&self, id: GameId) -> ServiceResult<Option<GameRecord>>;
    async fn get_games(&self, filter: GameFilter) -> ServiceResult<GameFilterResult>;
}

pub type ArcGameHistoryService = Arc<Box<dyn GameHistoryService + Send + Sync>>;
pub struct GameHistoryServiceImpl {
    game_repository: ArcGameRepository,
}

impl GameHistoryServiceImpl {
    pub fn new(game_repository: ArcGameRepository) -> Self {
        Self { game_repository }
    }
}

#[async_trait::async_trait]
impl GameHistoryService for GameHistoryServiceImpl {
    async fn get_game_record(&self, id: GameId) -> ServiceResult<Option<GameRecord>> {
        self.game_repository.get_game_record(id).await
    }

    async fn get_games(&self, filter: GameFilter) -> ServiceResult<GameFilterResult> {
        self.game_repository.get_games(filter).await
    }
}
