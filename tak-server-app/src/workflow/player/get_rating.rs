use std::sync::Arc;

use chrono::Utc;

use crate::{
    domain::{
        PaginatedResponse, PlayerId, RepoError, RepoRetrieveError,
        rating::{PlayerRating, RatingQuery, RatingRepository, RatingService},
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

pub struct PlayerGetRatingUseCaseImpl<R: RatingRepository, RS: RatingService> {
    rating_repository: Arc<R>,
    rating_service: Arc<RS>,
}

impl<R: RatingRepository, RS: RatingService> PlayerGetRatingUseCaseImpl<R, RS> {
    pub fn new(rating_repository: Arc<R>, rating_service: Arc<RS>) -> Self {
        Self {
            rating_repository,
            rating_service,
        }
    }
}

#[async_trait::async_trait]
impl<R: RatingRepository + Send + Sync + 'static, RS: RatingService + Send + Sync + 'static>
    PlayerGetRatingUseCase for PlayerGetRatingUseCaseImpl<R, RS>
{
    async fn query_ratings(
        &self,
        query: RatingQuery,
    ) -> Result<PaginatedResponse<RatingView>, RepoError> {
        let now = Utc::now();
        self.rating_repository
            .query_ratings(query)
            .await
            .map(|res| PaginatedResponse {
                total_count: res.total_count,
                items: res
                    .items
                    .into_iter()
                    .map(|rating| {
                        let participation_rating =
                            self.rating_service.get_current_rating(&rating, now);
                        RatingView::from(rating, participation_rating)
                    })
                    .collect(),
            })
    }

    async fn get_rating(&self, player_id: PlayerId) -> Option<RatingView> {
        let now = Utc::now();
        match self.rating_repository.get_player_rating(player_id).await {
            Ok(rating) => {
                let participation_rating = self.rating_service.get_current_rating(&rating, now);
                Some(RatingView::from(rating, participation_rating))
            }
            Err(RepoRetrieveError::NotFound) => {
                let default_rating = PlayerRating::new(player_id);
                let participation_rating =
                    self.rating_service.get_current_rating(&default_rating, now);
                Some(RatingView::from(default_rating, participation_rating))
            }
            Err(RepoRetrieveError::StorageError(e)) => {
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
