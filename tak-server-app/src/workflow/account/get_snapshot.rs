use std::sync::Arc;

use crate::{
    domain::{
        PlayerId, RepoRetrieveError,
        game_history::PlayerSnapshot,
        rating::{PlayerRating, RatingRepository, RatingService},
    },
    workflow::account::get_account::{GetAccountError, GetAccountWorkflow},
};

#[async_trait::async_trait]
pub trait GetSnapshotWorkflow {
    async fn get_snapshot(
        &self,
        player_id: PlayerId,
        date: chrono::DateTime<chrono::Utc>,
    ) -> PlayerSnapshot;
}

pub struct GetSnapshotWorkflowImpl<U: GetAccountWorkflow, R: RatingRepository, RS: RatingService> {
    get_account_workflow: Arc<U>,
    rating_repository: Arc<R>,
    rating_service: Arc<RS>,
}

impl<U: GetAccountWorkflow, R: RatingRepository, RS: RatingService>
    GetSnapshotWorkflowImpl<U, R, RS>
{
    pub fn new(
        get_account_workflow: Arc<U>,
        rating_repository: Arc<R>,
        rating_service: Arc<RS>,
    ) -> Self {
        Self {
            get_account_workflow,
            rating_repository,
            rating_service,
        }
    }
}

#[async_trait::async_trait]
impl<
    U: GetAccountWorkflow + Send + Sync + 'static,
    R: RatingRepository + Send + Sync + 'static,
    RS: RatingService + Send + Sync + 'static,
> GetSnapshotWorkflow for GetSnapshotWorkflowImpl<U, R, RS>
{
    async fn get_snapshot(
        &self,
        player_id: PlayerId,
        date: chrono::DateTime<chrono::Utc>,
    ) -> PlayerSnapshot {
        let username = match self.get_account_workflow.get_account(player_id).await {
            Ok(account) => Some(account.username),
            Err(GetAccountError::AccountNotFound) => None,
            Err(GetAccountError::RepositoryError) => {
                log::error!(
                    "Failed to retrieve account for player {}: Repository error",
                    player_id.to_string(),
                );
                None
            }
        };
        let current_rating = match self.rating_repository.get_player_rating(player_id).await {
            Ok(rating) => Some(self.rating_service.get_current_rating(&rating, date)),
            Err(RepoRetrieveError::NotFound) => {
                let default_rating = PlayerRating::new(player_id);
                Some(
                    self.rating_service
                        .get_current_rating(&default_rating, date),
                )
            }
            Err(RepoRetrieveError::StorageError(e)) => {
                log::error!(
                    "Failed to retrieve rating for player {}: {}",
                    player_id.to_string(),
                    e
                );
                None
            }
        };

        PlayerSnapshot::new(player_id, username, current_rating)
    }
}
