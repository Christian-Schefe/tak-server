use tak_domain::{UserId, rating::PlayerRating};

use crate::ports::{PaginatedResponse, Pagination, RepoError, RepoRetrieveError, SortOrder};

#[async_trait::async_trait]
pub trait RatingRepository {
    async fn get_player_rating(&self, user_id: UserId) -> Result<PlayerRating, RepoRetrieveError>;
    async fn get_player_ranking(
        &self,
        user_id: UserId,
    ) -> Result<(u32, PlayerRating), RepoRetrieveError>;
    async fn query_player_ratings(
        &self,
        query: RatingQuery,
    ) -> Result<PaginatedResponse<PlayerRating>, RepoError>;
    async fn update_player_ratings<R: Send + 'static>(
        &self,
        white: UserId,
        black: UserId,
        calc_fn: impl FnOnce(
            Option<PlayerRating>,
            Option<PlayerRating>,
        ) -> (PlayerRating, PlayerRating, R)
        + Send
        + 'static,
    ) -> Result<R, RepoError>;
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
