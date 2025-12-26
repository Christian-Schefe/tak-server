use std::sync::Arc;

use crate::{
    domain::{
        PlayerId, RepoError,
        game_history::PlayerSnapshot,
        rating::{PlayerRating, RatingRepository, RatingService},
    },
    workflow::account::get_username::GetUsernameWorkflow,
};

#[async_trait::async_trait]
pub trait GetSnapshotWorkflow {
    async fn get_snapshot(
        &self,
        player_id: PlayerId,
        date: chrono::DateTime<chrono::Utc>,
    ) -> PlayerSnapshot;
}

pub struct GetSnapshotWorkflowImpl<U: GetUsernameWorkflow, R: RatingRepository, RS: RatingService> {
    get_username_workflow: Arc<U>,
    rating_repository: Arc<R>,
    rating_service: Arc<RS>,
}

impl<U: GetUsernameWorkflow, R: RatingRepository, RS: RatingService>
    GetSnapshotWorkflowImpl<U, R, RS>
{
    pub fn new(
        get_username_workflow: Arc<U>,
        rating_repository: Arc<R>,
        rating_service: Arc<RS>,
    ) -> Self {
        Self {
            get_username_workflow,
            rating_repository,
            rating_service,
        }
    }
}

#[async_trait::async_trait]
impl<
    U: GetUsernameWorkflow + Send + Sync + 'static,
    R: RatingRepository + Send + Sync + 'static,
    RS: RatingService + Send + Sync + 'static,
> GetSnapshotWorkflow for GetSnapshotWorkflowImpl<U, R, RS>
{
    async fn get_snapshot(
        &self,
        player_id: PlayerId,
        date: chrono::DateTime<chrono::Utc>,
    ) -> PlayerSnapshot {
        let username = self.get_username_workflow.get_username(player_id).await;
        let current_rating = match self
            .rating_repository
            .get_or_create_player_rating(player_id, || PlayerRating::new())
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
