use std::sync::Arc;

use crate::{
    domain::{
        PlayerId, RepoRetrieveError,
        profile::{AccountProfile, AccountProfileRepository},
    },
    ports::player_mapping::PlayerAccountMappingRepository,
};

#[async_trait::async_trait]
pub trait GetProfileUseCase {
    async fn get_profile(&self, player_id: PlayerId) -> Result<AccountProfile, GetProfileError>;
}

pub enum GetProfileError {
    ProfileNotFound,
    RepositoryError,
}

pub struct GetProfileUseCaseImpl<P: PlayerAccountMappingRepository, PF: AccountProfileRepository> {
    player_account_mapping_repo: Arc<P>,
    profile_information_repo: Arc<PF>,
}

impl<P: PlayerAccountMappingRepository, PF: AccountProfileRepository> GetProfileUseCaseImpl<P, PF> {
    pub fn new(player_account_mapping_repo: Arc<P>, profile_information_repo: Arc<PF>) -> Self {
        Self {
            player_account_mapping_repo,
            profile_information_repo,
        }
    }
}

#[async_trait::async_trait]
impl<
    P: PlayerAccountMappingRepository + Send + Sync + 'static,
    PF: AccountProfileRepository + Send + Sync + 'static,
> GetProfileUseCase for GetProfileUseCaseImpl<P, PF>
{
    async fn get_profile(&self, player_id: PlayerId) -> Result<AccountProfile, GetProfileError> {
        let account_id = match self
            .player_account_mapping_repo
            .get_account_id(player_id)
            .await
        {
            Ok(account_id) => account_id,
            Err(RepoRetrieveError::NotFound) => return Err(GetProfileError::ProfileNotFound),
            Err(RepoRetrieveError::StorageError(e)) => {
                log::error!(
                    "Failed to retrieve account ID for player {}: {}",
                    player_id,
                    e
                );
                return Err(GetProfileError::RepositoryError);
            }
        };

        match self
            .profile_information_repo
            .get_profile_information(&account_id)
            .await
        {
            Ok(profile_information) => Ok(profile_information),
            Err(RepoRetrieveError::NotFound) => Err(GetProfileError::ProfileNotFound),
            Err(RepoRetrieveError::StorageError(e)) => {
                log::error!(
                    "Failed to retrieve profile information for account {}: {}",
                    account_id,
                    e
                );
                Err(GetProfileError::RepositoryError)
            }
        }
    }
}
