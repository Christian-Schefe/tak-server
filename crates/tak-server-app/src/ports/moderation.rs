use tak_domain::{UserId, moderation::Moderation};

use crate::ports::transaction::{TransactionError, TransactionItem};

pub trait ModerationRepository<TrCtx> {
    fn get_moderation(
        &self,
        user_id: UserId,
    ) -> TransactionItem<TrCtx, Result<Option<Moderation>, TransactionError>>;
    fn set_moderation(
        &self,
        user_id: UserId,
        moderation: Moderation,
    ) -> TransactionItem<TrCtx, Result<(), TransactionError>>;
}
