use chrono::{DateTime, Utc};
use tak_core::{TakGameResult, TakPlayer};
use tak_domain::{
    GameId, MatchId, UserId,
    game::GameEvent,
    game_history::{GameRatingInfo, GameRecord},
};

use crate::ports::{PaginatedResponse, Pagination, RepoError, RepoRetrieveError, SortOrder};

#[async_trait::async_trait]
pub trait GameRepository {
    async fn save_ongoing_game(&self, game: GameRecord) -> Result<GameId, RepoError>;
    async fn update_finished_game(
        &self,
        game_id: GameId,
        result: TakGameResult,
        events: Vec<GameEvent>,
        rating_info: Option<GameRatingInfo>,
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
