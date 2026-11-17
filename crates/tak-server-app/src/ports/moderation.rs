use tak_domain::{UserId, moderation::Moderation};

use crate::errors::{AppResult, InternalError};

#[async_trait::async_trait]
pub trait ModerationRepository<RpCtx> {
    async fn get_moderation(
        &self,
        ctx: &RpCtx,
        user_id: UserId,
    ) -> AppResult<Option<Moderation>, (InternalError,)>;
    async fn set_moderation(
        &self,
        ctx: &RpCtx,
        user_id: UserId,
        moderation: Moderation,
    ) -> AppResult<(), (InternalError,)>;
}
