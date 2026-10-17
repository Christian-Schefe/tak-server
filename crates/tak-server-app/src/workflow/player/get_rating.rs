use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::{
    ports::{
        PaginatedResponse,
        rating::{RatingQuery, RatingRepository},
        stats::RatingHistoryRepository,
    },
    workflow::player::{RatingHistoryEntryView, RatingHistoryRangeView},
};
use tak_domain::UserId;

#[async_trait::async_trait]
pub trait PlayerGetRatingUseCase {
    async fn get_rating_history(
        &self,
        user_id: UserId,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Result<RatingHistoryRangeView, GetRatingError>;
    async fn query_player_ratings(
        &self,
        query: RatingQuery,
    ) -> Result<PaginatedResponse<(UserId, f64)>, GetRatingError>;
}

pub struct PlayerGetRatingUseCaseImpl<R: RatingRepository, RH: RatingHistoryRepository> {
    rating_repository: Arc<R>,
    rating_history_repository: Arc<RH>,
}

impl<R: RatingRepository, RH: RatingHistoryRepository> PlayerGetRatingUseCaseImpl<R, RH> {
    pub fn new(rating_repository: Arc<R>, rating_history_repository: Arc<RH>) -> Self {
        Self {
            rating_repository,
            rating_history_repository,
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
> PlayerGetRatingUseCase for PlayerGetRatingUseCaseImpl<R, RH>
{
    async fn get_rating_history(
        &self,
        user_id: UserId,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Result<RatingHistoryRangeView, GetRatingError> {
        let rating_history_range = self
            .rating_history_repository
            .get_rating_history(user_id, from, to)
            .await
            .map_err(|e| {
                tracing::error!(
                    "Error retrieving rating history for player {}: {}",
                    user_id,
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

    async fn query_player_ratings(
        &self,
        query: RatingQuery,
    ) -> Result<PaginatedResponse<(UserId, f64)>, GetRatingError> {
        let paginated_ratings = self
            .rating_repository
            .query_player_ratings(query)
            .await
            .map_err(|e| {
                tracing::error!("Error querying player ratings: {}", e);
                GetRatingError::Internal
            })?;
        Ok(PaginatedResponse {
            total_count: paginated_ratings.total_count,
            items: paginated_ratings
                .items
                .into_iter()
                .map(|r| (r.user_id, r.rating))
                .collect(),
        })
    }
}
