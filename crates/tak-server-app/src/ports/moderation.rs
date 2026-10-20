use tak_domain::{UserId, moderation::Moderation};

use crate::ports::transaction::RepoError;

#[async_trait::async_trait]
pub trait ModerationRepository<RpCtx> {
    async fn get_moderation(
        &self,
        ctx: &RpCtx,
        user_id: UserId,
    ) -> Result<Option<Moderation>, RepoError>;
    async fn set_moderation(
        &self,
        ctx: &RpCtx,
        user_id: UserId,
        moderation: Moderation,
    ) -> Result<(), RepoError>;
}
