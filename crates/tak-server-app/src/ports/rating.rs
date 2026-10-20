use tak_domain::{UserId, rating::PlayerRating};

use crate::ports::{PaginatedResponse, Pagination, SortOrder, transaction::RepoError};

#[async_trait::async_trait]
pub trait RatingRepository<RpCtx> {
    async fn get_player_rating(
        &self,
        ctx: &RpCtx,
        user_id: UserId,
    ) -> Result<Option<PlayerRating>, RepoError>;
    async fn get_player_ranking(
        &self,
        ctx: &RpCtx,
        user_id: UserId,
    ) -> Result<Option<(u32, PlayerRating)>, RepoError>;
    async fn query_player_ratings(
        &self,
        ctx: &RpCtx,
        query: RatingQuery,
    ) -> Result<PaginatedResponse<PlayerRating>, RepoError>;
    async fn set_player_rating(
        &self,
        ctx: &RpCtx,
        user_id: UserId,
        rating: PlayerRating,
    ) -> Result<(), RepoError>;
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
