use tak_domain::{UserId, rating::PlayerRating};

use crate::ports::{
    PaginatedResponse, Pagination, SortOrder,
    transaction::{TransactionError, TransactionItem},
};

pub trait RatingRepository<TrCtx> {
    fn get_player_rating(
        &self,
        user_id: UserId,
    ) -> TransactionItem<TrCtx, Result<Option<PlayerRating>, TransactionError>>;
    fn get_player_ranking(
        &self,
        user_id: UserId,
    ) -> TransactionItem<TrCtx, Result<Option<(u32, PlayerRating)>, TransactionError>>;
    fn query_player_ratings(
        &self,
        query: RatingQuery,
    ) -> TransactionItem<TrCtx, Result<PaginatedResponse<PlayerRating>, TransactionError>>;
    fn set_player_rating(
        &self,
        user_id: UserId,
        rating: PlayerRating,
    ) -> TransactionItem<TrCtx, Result<(), TransactionError>>;
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
