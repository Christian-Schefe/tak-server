use std::sync::Arc;

use chrono::Utc;

use crate::{
    domain::{
        PaginatedResponse, PlayerId, RepoError, RepoRetrieveError,
        rating::{RatingQuery, RatingRepository, RatingService},
    },
    workflow::player::RatedPlayerView,
};

#[async_trait::async_trait]
pub trait PlayerGetRatingUseCase {
    async fn query_ratings(
        &self,
        query: RatingQuery,
    ) -> Result<PaginatedResponse<RatedPlayerView>, RepoError>;
    async fn get_rating(
        &self,
        player_id: PlayerId,
    ) -> Result<Option<RatedPlayerView>, GetRatingError>;
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

pub enum GetRatingError {
    Internal,
}

#[async_trait::async_trait]
impl<R: RatingRepository + Send + Sync + 'static, RS: RatingService + Send + Sync + 'static>
    PlayerGetRatingUseCase for PlayerGetRatingUseCaseImpl<R, RS>
{
    async fn query_ratings(
        &self,
        query: RatingQuery,
    ) -> Result<PaginatedResponse<RatedPlayerView>, RepoError> {
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
                        RatedPlayerView::from(rating, participation_rating)
                    })
                    .collect(),
            })
    }

    async fn get_rating(
        &self,
        player_id: PlayerId,
    ) -> Result<Option<RatedPlayerView>, GetRatingError> {
        let now = Utc::now();
        match self.rating_repository.get_player_rating(player_id).await {
            Ok(rating) => {
                let participation_rating = self.rating_service.get_current_rating(&rating, now);
                Ok(Some(RatedPlayerView::from(rating, participation_rating)))
            }
            Err(RepoRetrieveError::NotFound) => Ok(None),
            Err(RepoRetrieveError::StorageError(e)) => {
                log::error!(
                    "Error retrieving player rating for player {}: {}",
                    player_id,
                    e
                );
                Err(GetRatingError::Internal)
            }
        }
    }
}
