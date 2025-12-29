use std::sync::Arc;

use crate::{
    domain::{
        PaginatedResponse, PlayerId, RepoError,
        rating::{PlayerRating, RatingQuery, RatingRepository},
    },
    workflow::player::RatingView,
};

#[async_trait::async_trait]
pub trait PlayerGetRatingUseCase {
    async fn query_ratings(
        &self,
        query: RatingQuery,
    ) -> Result<PaginatedResponse<RatingView>, RepoError>;
    async fn get_rating(&self, player_id: PlayerId) -> Option<RatingView>;
}

pub struct PlayerGetRatingUseCaseImpl<R: RatingRepository> {
    rating_repository: Arc<R>,
}

impl<R: RatingRepository> PlayerGetRatingUseCaseImpl<R> {
    pub fn new(rating_repository: Arc<R>) -> Self {
        Self { rating_repository }
    }
}

#[async_trait::async_trait]
impl<R: RatingRepository + Send + Sync + 'static> PlayerGetRatingUseCase
    for PlayerGetRatingUseCaseImpl<R>
{
    async fn query_ratings(
        &self,
        query: RatingQuery,
    ) -> Result<PaginatedResponse<RatingView>, RepoError> {
        self.rating_repository
            .query_ratings(query)
            .await
            .map(|res| PaginatedResponse {
                total_count: res.total_count,
                items: res
                    .items
                    .into_iter()
                    .map(|rating| RatingView::from(rating))
                    .collect(),
            })
    }
    async fn get_rating(&self, player_id: PlayerId) -> Option<RatingView> {
        match self
            .rating_repository
            .get_or_create_player_rating(player_id, move || PlayerRating::new(player_id))
            .await
        {
            Ok(rating) => Some(RatingView::from(rating)),
            Err(RepoError::StorageError(e)) => {
                log::error!(
                    "Error retrieving player rating for player {}: {}",
                    player_id,
                    e
                );
                None
            }
        }
    }
}
