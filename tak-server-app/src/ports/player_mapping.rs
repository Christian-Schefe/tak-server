use crate::domain::{AccountId, PlayerId, RepoError, RepoRetrieveError};

#[async_trait::async_trait]
pub trait PlayerAccountMappingRepository {
    async fn get_or_create_player_id(
        &self,
        account_id: &AccountId,
        create_fn: impl FnOnce() -> PlayerId + Send + 'static,
    ) -> Result<PlayerId, RepoError>;
    async fn get_account_id(&self, player_id: PlayerId) -> Result<AccountId, RepoRetrieveError>;
}
