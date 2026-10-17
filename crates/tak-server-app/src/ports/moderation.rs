use tak_domain::{UserId, moderation::Moderation};

use crate::ports::{RepoError, RepoRetrieveError};

#[async_trait::async_trait]
pub trait ModerationRepository {
    async fn get_moderation(&self, user_id: UserId) -> Result<Moderation, RepoRetrieveError>;
    async fn set_moderation(
        &self,
        user_id: UserId,
        moderation: Moderation,
    ) -> Result<(), RepoError>;
}
