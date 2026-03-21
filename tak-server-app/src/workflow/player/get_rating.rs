use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::{
    domain::{
        PaginatedResponse, PlayerId, RepoRetrieveError,
        rating::{RatingQuery, RatingRepository, RatingService},
        stats::RatingHistoryRepository,
    },
    workflow::player::{RatedPlayerView, RatingHistoryEntryView, RatingHistoryRangeView},
};

#[async_trait::async_trait]
pub trait PlayerGetRatingUseCase {
    async fn query_ratings(
        &self,
        query: RatingQuery,
    ) -> Result<PaginatedResponse<RatedPlayerView>, GetRatingError>;
    async fn get_rating(
        &self,
        player_id: PlayerId,
    ) -> Result<Option<RatedPlayerView>, GetRatingError>;
    async fn get_rating_history(
        &self,
        player_id: PlayerId,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Result<RatingHistoryRangeView, GetRatingError>;
}

pub struct PlayerGetRatingUseCaseImpl<
    R: RatingRepository,
    RH: RatingHistoryRepository,
    RS: RatingService,
> {
    rating_repository: Arc<R>,
    rating_history_repository: Arc<RH>,
    rating_service: Arc<RS>,
}

impl<R: RatingRepository, RH: RatingHistoryRepository, RS: RatingService>
    PlayerGetRatingUseCaseImpl<R, RH, RS>
{
    pub fn new(
        rating_repository: Arc<R>,
        rating_history_repository: Arc<RH>,
        rating_service: Arc<RS>,
    ) -> Self {
        Self {
            rating_repository,
            rating_history_repository,
            rating_service,
        }
    }
}

pub enum GetRatingError {
    Internal,
}

#[async_trait::async_trait]
impl<
    R: RatingRepository + Send + Sync + 'static,
    RH: RatingHistoryRepository + Send + Sync + 'static,
    RS: RatingService + Send + Sync + 'static,
> PlayerGetRatingUseCase for PlayerGetRatingUseCaseImpl<R, RH, RS>
{
    async fn query_ratings(
        &self,
        query: RatingQuery,
    ) -> Result<PaginatedResponse<RatedPlayerView>, GetRatingError> {
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
            .map_err(|e| {
                tracing::error!("Error querying ratings: {}", e);
                GetRatingError::Internal
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
                tracing::error!(
                    "Error retrieving player rating for player {}: {}",
                    player_id,
                    e
                );
                Err(GetRatingError::Internal)
            }
        }
    }

    async fn get_rating_history(
        &self,
        player_id: PlayerId,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Result<RatingHistoryRangeView, GetRatingError> {
        let rating_history_range = self
            .rating_history_repository
            .get_rating_history(player_id, from, to)
            .await
            .map_err(|e| {
                tracing::error!(
                    "Error retrieving rating history for player {}: {}",
                    player_id,
                    e
                );
                GetRatingError::Internal
            })?;
        Ok(RatingHistoryRangeView {
            entries: rating_history_range
                .entries
                .into_iter()
                .map(|entry| RatingHistoryEntryView {
                    timestamp: entry.timestamp,
                    rating: entry.rating,
                })
                .collect(),
            first_entry_before_range: rating_history_range.first_entry_before_range.map(|entry| {
                RatingHistoryEntryView {
                    timestamp: entry.timestamp,
                    rating: entry.rating,
                }
            }),
        })
    }
}
