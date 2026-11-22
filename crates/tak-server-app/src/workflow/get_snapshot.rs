use crate::{
    PortsCollection,
    errors::{AppResult, InternalError},
    ports::{authentication::AccountPort, rating::RatingRepository},
};
use std::sync::Arc;
use tak_domain::{UserId, game_record::PlayerSnapshot};
use better_errors::OneOf;

#[async_trait::async_trait]
pub trait GetSnapshotWorkflow<RpCtx> {
    async fn get_snapshot(
        &self,
        ctx: &RpCtx,
        user_id: UserId,
    ) -> AppResult<Option<PlayerSnapshot>, (InternalError,)>;
}

pub struct GetSnapshotWorkflowImpl<Ports: PortsCollection, A: AccountPort> {
    ports: Arc<Ports>,
    account_info_service: Arc<A>,
}

impl<Ports: PortsCollection, A: AccountPort> GetSnapshotWorkflowImpl<Ports, A> {
    pub fn new(ports: Arc<Ports>, account_info_service: Arc<A>) -> Self {
        Self {
            ports,
            account_info_service,
        }
    }
}

#[async_trait::async_trait]
impl<Ports: PortsCollection + Send + Sync + 'static, A: AccountPort + Send + Sync + 'static>
    GetSnapshotWorkflow<Ports::TrCtx> for GetSnapshotWorkflowImpl<Ports, A>
{
    async fn get_snapshot(
        &self,
        ctx: &Ports::TrCtx,
        user_id: UserId,
    ) -> AppResult<Option<PlayerSnapshot>, (InternalError,)> {
        let Some(account) = self
            .account_info_service
            .get_account_info(user_id)
            .await
            .map_err(OneOf::broaden)?
        else {
            return Ok(None);
        };
        let current_rating = match self
            .ports
            .rating_repository()
            .get_player_rating(ctx, user_id)
            .await
            .map_err(OneOf::broaden)?
        {
            Some(rating) => Some(rating.rating),
            None => None,
        };

        Ok(Some(PlayerSnapshot::new(
            user_id,
            account.username,
            current_rating,
        )))
    }
}
