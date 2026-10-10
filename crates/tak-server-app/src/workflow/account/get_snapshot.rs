use std::sync::Arc;

use crate::{
    domain::{
        RepoRetrieveError, UserId,
        game_history::PlayerSnapshot,
        rating::{PlayerRating, RatingRepository, RatingService},
    },
    ports::authentication::AuthenticationPort,
};

#[async_trait::async_trait]
pub trait GetSnapshotWorkflow {
    async fn get_snapshot(
        &self,
        user_id: UserId,
        date: chrono::DateTime<chrono::Utc>,
    ) -> PlayerSnapshot;
}

pub struct GetSnapshotWorkflowImpl<A: AuthenticationPort, R: RatingRepository, RS: RatingService> {
    authentication_service: Arc<A>,
    rating_repository: Arc<R>,
    rating_service: Arc<RS>,
}

impl<A: AuthenticationPort, R: RatingRepository, RS: RatingService>
    GetSnapshotWorkflowImpl<A, R, RS>
{
    pub fn new(authentication_service: Arc<A>, rating_repository: Arc<R>, rating_service: Arc<RS>) -> Self {
        Self {
            authentication_service,
            rating_repository,
            rating_service,
        }
    }
}

#[async_trait::async_trait]
impl<
    A: AuthenticationPort + Send + Sync + 'static,
    R: RatingRepository + Send + Sync + 'static,
    RS: RatingService + Send + Sync + 'static,
> GetSnapshotWorkflow for GetSnapshotWorkflowImpl<A, R, RS>
{
    async fn get_snapshot(
        &self,
        user_id: UserId,
        date: chrono::DateTime<chrono::Utc>,
    ) -> PlayerSnapshot {
        let username = match self.authentication_service.get_account(user_id).await {
            Ok(Some(account)) => Some(account.username),
            Ok(None) => None,
            Err(_) => {
                tracing::error!(
                    "Failed to retrieve account for player {}: Repository error",
                    user_id.to_string(),
                );
                None
            }
        };
        let current_rating = match self.rating_repository.get_player_rating(user_id).await {
            Ok(rating) => Some(self.rating_service.get_current_rating(&rating, date)),
            Err(RepoRetrieveError::NotFound) => {
                let default_rating = PlayerRating::new(user_id);
                Some(
                    self.rating_service
                        .get_current_rating(&default_rating, date),
                )
            }
            Err(RepoRetrieveError::StorageError(e)) => {
                tracing::error!(
                    "Failed to retrieve rating for player {}: {}",
                    user_id.to_string(),
                    e
                );
                None
            }
        };

        PlayerSnapshot::new(username, current_rating)
    }
}
