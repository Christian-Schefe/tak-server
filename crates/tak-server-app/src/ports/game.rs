use chrono::{DateTime, Utc};
use tak_core::{TakGameResult, TakPlayer};
use tak_domain::{
    GameId, MatchId, UserId,
    game::{Game, OngoingGame},
};

use crate::ports::{PaginatedResponse, Pagination, SortOrder, transaction::RepoError};

#[async_trait::async_trait]
pub trait GameRepository<RpCtx> {
    async fn get_game(&self, ctx: &RpCtx, game_id: GameId) -> Result<Option<Game>, RepoError>;
    async fn create_game(&self, ctx: &RpCtx, game: Game) -> Result<GameId, RepoError>;
    async fn update_game(&self, ctx: &RpCtx, game_id: GameId, game: Game) -> Result<(), RepoError>;
    async fn get_games_by_player(
        &self,
        ctx: &RpCtx,
        user_id: UserId,
    ) -> Result<Vec<(GameId, Game)>, RepoError>;
    async fn get_ongoing_games(&self, ctx: &RpCtx)
    -> Result<Vec<(GameId, OngoingGame)>, RepoError>;

    async fn query_games(
        &self,
        ctx: &RpCtx,
        query: GameQuery,
    ) -> Result<PaginatedResponse<(GameId, Game)>, RepoError>;
    async fn get_games_of_match(
        &self,
        ctx: &RpCtx,
        match_id: MatchId,
    ) -> Result<Vec<(GameId, Game)>, RepoError>;
}

#[derive(Debug, Clone, Default)]
pub struct GameQuery {
    pub id_selector: Option<GameIdSelector>,
    pub date_selector: Option<DateSelector>,
    pub player_filters: Vec<(GamePlayerFilter, Option<TakPlayer>)>,
    pub game_results: Vec<Option<TakGameResult>>,
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
