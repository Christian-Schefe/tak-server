use tak_domain::{UserId, rating::PlayerRating};

use crate::{
    errors::{AppResult, InternalError},
    ports::{PaginatedResponse, Pagination, SortOrder},
};

#[async_trait::async_trait]
pub trait RatingRepository<RpCtx> {
    async fn get_player_rating(
        &self,
        ctx: &RpCtx,
        user_id: UserId,
    ) -> AppResult<Option<PlayerRating>, (InternalError,)>;
    async fn get_player_ranking(
        &self,
        ctx: &RpCtx,
        user_id: UserId,
    ) -> AppResult<Option<(u32, PlayerRating)>, (InternalError,)>;
    async fn query_player_ratings(
        &self,
        ctx: &RpCtx,
        query: RatingQuery,
    ) -> AppResult<PaginatedResponse<PlayerRating>, (InternalError,)>;
    async fn set_player_rating(
        &self,
        ctx: &RpCtx,
        user_id: UserId,
        rating: PlayerRating,
    ) -> AppResult<(), (InternalError,)>;
}

#[derive(Debug, Clone)]
pub enum RatingSortBy {
    Rating,
    RatedGames,
    MaxRating,
}

#[derive(Debug, Clone, Default)]
pub struct RatingQuery {
    pub pagination: Pagination,
    pub sort: Option<(SortOrder, RatingSortBy)>,
}
