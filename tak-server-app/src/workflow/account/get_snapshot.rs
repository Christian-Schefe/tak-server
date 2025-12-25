use std::sync::Arc;

use crate::{
    domain::{
        PlayerId,
        game_history::PlayerSnapshot,
        player::PlayerRepository,
        rating::{RatingRepository, RatingService},
    },
    ports::authentication::{AuthSubject, AuthenticationService},
};

#[async_trait::async_trait]
pub trait GetSnapshotWorkflow {
    async fn get_username(&self, player_id: PlayerId) -> Option<String>;
    async fn get_snapshot(
        &self,
        player_id: PlayerId,
        date: chrono::DateTime<chrono::Utc>,
    ) -> PlayerSnapshot;
}

pub struct GetSnapshotWorkflowImpl<
    A: AuthenticationService,
    P: PlayerRepository,
    R: RatingRepository,
    RS: RatingService,
> {
    authentication_service: Arc<A>,
    player_repository: Arc<P>,
    rating_repository: Arc<R>,
    rating_service: Arc<RS>,
}

impl<A: AuthenticationService, P: PlayerRepository, R: RatingRepository, RS: RatingService>
    GetSnapshotWorkflowImpl<A, P, R, RS>
{
    pub fn new(
        authentication_service: Arc<A>,
        player_repository: Arc<P>,
        rating_repository: Arc<R>,
        rating_service: Arc<RS>,
    ) -> Self {
        Self {
            authentication_service,
            player_repository,
            rating_repository,
            rating_service,
        }
    }
}

#[async_trait::async_trait]
impl<
    A: AuthenticationService + Send + Sync + 'static,
    P: PlayerRepository + Send + Sync + 'static,
    R: RatingRepository + Send + Sync + 'static,
    RS: RatingService + Send + Sync + 'static,
> GetSnapshotWorkflow for GetSnapshotWorkflowImpl<A, P, R, RS>
{
    async fn get_username(&self, player_id: PlayerId) -> Option<String> {
        let account_id = self
            .player_repository
            .get_account_id_for_player(player_id)
            .await?;
        let subject = self.authentication_service.get_subject(account_id)?;
        match subject.subject_type {
            AuthSubject::Player { username, .. } => Some(username),
            AuthSubject::Bot { username } => Some(username),
            AuthSubject::Guest { guest_number } => Some(format!("Guest{}", guest_number)),
        }
    }

    async fn get_snapshot(
        &self,
        player_id: PlayerId,
        date: chrono::DateTime<chrono::Utc>,
    ) -> PlayerSnapshot {
        let username = self.get_username(player_id).await;
        let rating = self.rating_repository.get_player_rating(player_id).await;
        let current_rating = self.rating_service.get_current_rating(&rating, date);
        PlayerSnapshot::new(player_id, username, Some(current_rating))
    }
}
