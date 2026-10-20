use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::{
    PortsCollection,
    ports::{
        PaginatedResponse,
        rating::{RatingQuery, RatingRepository},
        stats::RatingHistoryRepository,
        transaction::{RepoError, TransactionManager},
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

pub struct PlayerGetRatingUseCaseImpl<Ports: PortsCollection> {
    ports: Arc<Ports>,
}

impl<Ports: PortsCollection> PlayerGetRatingUseCaseImpl<Ports> {
    pub fn new(ports: Arc<Ports>) -> Self {
        Self { ports }
    }
}

pub enum GetRatingError {
    Internal,
}

#[async_trait::async_trait]
impl<Ports: PortsCollection + Send + Sync + 'static> PlayerGetRatingUseCase
    for PlayerGetRatingUseCaseImpl<Ports>
{
    async fn get_rating_history(
        &self,
        user_id: UserId,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Result<RatingHistoryRangeView, GetRatingError> {
        let ctx = self.ports.transaction_manager().simple();
        let rating_history_range = self
            .ports
            .rating_history_repository()
            .get_rating_history(&ctx, user_id, from, to)
            .await
            .map_err(|RepoError(e)| {
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
        let ctx = self.ports.transaction_manager().simple();
        let paginated_ratings = self
            .ports
            .rating_repository()
            .query_player_ratings(&ctx, query)
            .await
            .map_err(|RepoError(e)| {
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
