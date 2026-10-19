use crate::{
    PortsCollection,
    ports::{
        authentication::AccountPort,
        rating::RatingRepository,
        transaction::{TransactionError, TransactionManager},
    },
};
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

pub struct GetSnapshotWorkflowImpl<Ports: PortsCollection, A: AccountPort, RS: RatingService> {
    ports: Arc<Ports>,
    account_info_service: Arc<A>,
    rating_service: Arc<RS>,
}

impl<Ports: PortsCollection, A: AccountPort, RS: RatingService>
    GetSnapshotWorkflowImpl<Ports, A, RS>
{
    pub fn new(ports: Arc<Ports>, account_info_service: Arc<A>, rating_service: Arc<RS>) -> Self {
        Self {
            ports,
            account_info_service,
            rating_service,
        }
    }
}

#[async_trait::async_trait]
impl<
    Ports: PortsCollection + Send + Sync + 'static,
    A: AccountPort + Send + Sync + 'static,
    RS: RatingService + Send + Sync + 'static,
> GetSnapshotWorkflow for GetSnapshotWorkflowImpl<Ports, A, RS>
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
        let current_rating = match self
            .ports
            .transaction_manager()
            .execute(self.ports.rating_repository().get_player_rating(user_id))
            .await
        {
            Ok(Some(rating)) => Some(self.rating_service.get_current_rating(&rating, date)),
            Ok(None) => None,
            Err(TransactionError(e)) => {
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
