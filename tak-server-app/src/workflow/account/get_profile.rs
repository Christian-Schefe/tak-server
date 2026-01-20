use std::sync::Arc;

use crate::{
    domain::{AccountId, RepoRetrieveError, profile::AccountProfileRepository},
    workflow::account::AccountProfileView,
};

#[async_trait::async_trait]
pub trait GetProfileUseCase {
    async fn get_profile(
        &self,
        account_id: AccountId,
    ) -> Result<AccountProfileView, GetProfileError>;
}

pub enum GetProfileError {
    RepositoryError,
}

pub struct GetProfileUseCaseImpl<PF: AccountProfileRepository> {
    profile_information_repo: Arc<PF>,
}

impl<PF: AccountProfileRepository> GetProfileUseCaseImpl<PF> {
    pub fn new(profile_information_repo: Arc<PF>) -> Self {
        Self {
            profile_information_repo,
        }
    }
}

#[async_trait::async_trait]
impl<PF: AccountProfileRepository + Send + Sync + 'static> GetProfileUseCase
    for GetProfileUseCaseImpl<PF>
{
    async fn get_profile(
        &self,
        account_id: AccountId,
    ) -> Result<AccountProfileView, GetProfileError> {
        match self
            .profile_information_repo
            .get_profile_information(&account_id)
            .await
        {
            Ok(profile_information) => Ok(profile_information.into()),
            Err(RepoRetrieveError::NotFound) => Ok(AccountProfileView { country: None }),
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
