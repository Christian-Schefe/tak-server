use std::sync::Arc;

use crate::{
    domain::{
        PlayerId, RepoError,
        game_history::PlayerSnapshot,
        rating::{PlayerRating, RatingRepository, RatingService},
    },
    workflow::account::get_account::GetAccountWorkflow,
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
        let account = self.get_account_workflow.get_account(player_id).await.ok();
        let username = account.map(|a| a.username);
        let current_rating = match self
            .rating_repository
            .get_or_create_player_rating(player_id, move || PlayerRating::new(player_id))
            .await
        {
            Ok(rating) => Some(self.rating_service.get_current_rating(&rating, date)),
            Err(RepoError::StorageError(e)) => {
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
