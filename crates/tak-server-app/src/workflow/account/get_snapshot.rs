use crate::ports::{RepoRetrieveError, authentication::AccountPort, rating::RatingRepository};
use std::sync::Arc;
use tak_domain::{UserId, game_history::PlayerSnapshot, rating::RatingService};

pub enum GetSnapshotError {
    NotFound,
    Internal,
}

#[async_trait::async_trait]
pub trait GetSnapshotWorkflow {
    async fn get_snapshot(
        &self,
        user_id: UserId,
        date: chrono::DateTime<chrono::Utc>,
    ) -> Result<PlayerSnapshot, GetSnapshotError>;
}

pub struct GetSnapshotWorkflowImpl<A: AccountPort, R: RatingRepository, RS: RatingService> {
    account_info_service: Arc<A>,
    rating_repository: Arc<R>,
    rating_service: Arc<RS>,
}

impl<A: AccountPort, R: RatingRepository, RS: RatingService> GetSnapshotWorkflowImpl<A, R, RS> {
    pub fn new(
        account_info_service: Arc<A>,
        rating_repository: Arc<R>,
        rating_service: Arc<RS>,
    ) -> Self {
        Self {
            account_info_service,
            rating_repository,
            rating_service,
        }
    }
}

#[async_trait::async_trait]
impl<
    A: AccountPort + Send + Sync + 'static,
    R: RatingRepository + Send + Sync + 'static,
    RS: RatingService + Send + Sync + 'static,
> GetSnapshotWorkflow for GetSnapshotWorkflowImpl<A, R, RS>
{
    async fn get_snapshot(
        &self,
        user_id: UserId,
        date: chrono::DateTime<chrono::Utc>,
    ) -> Result<PlayerSnapshot, GetSnapshotError> {
        let username = match self.account_info_service.get_account_info(user_id).await {
            Ok(Some(info)) => info.username,
            Ok(None) => return Err(GetSnapshotError::NotFound),
            Err(_) => {
                tracing::error!(
                    "Failed to retrieve account for player {}: Repository error",
                    user_id.to_string(),
                );
                return Err(GetSnapshotError::Internal);
            }
        };
        let current_rating = match self.rating_repository.get_player_rating(user_id).await {
            Ok(rating) => Some(self.rating_service.get_current_rating(&rating, date)),
            Err(RepoRetrieveError::NotFound) => None,
            Err(RepoRetrieveError::StorageError(e)) => {
                tracing::error!(
                    "Failed to retrieve rating for player {}: {}",
                    user_id.to_string(),
                    e
                );
                return Err(GetSnapshotError::Internal);
            }
        };

        Ok(PlayerSnapshot::new(username, current_rating))
    }
}
