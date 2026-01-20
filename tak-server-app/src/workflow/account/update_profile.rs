use std::sync::Arc;

use country_code_enum::CountryCode;

use crate::domain::{
    AccountId, RepoError,
    profile::{AccountProfile, AccountProfileRepository},
};

#[async_trait::async_trait]
pub trait UpdateProfileUseCase {
    async fn update_profile(
        &self,
        account_id: AccountId,
        country: Option<CountryCode>,
    ) -> Result<(), UpdateProfileError>;
}

pub enum UpdateProfileError {
    ProfileNotFound,
    RepositoryError,
}

pub struct UpdateProfileUseCaseImpl<PF: AccountProfileRepository> {
    profile_information_repo: Arc<PF>,
}

impl<PF: AccountProfileRepository> UpdateProfileUseCaseImpl<PF> {
    pub fn new(profile_information_repo: Arc<PF>) -> Self {
        Self {
            profile_information_repo,
        }
    }
}

#[async_trait::async_trait]
impl<PF: AccountProfileRepository + Send + Sync + 'static> UpdateProfileUseCase
    for UpdateProfileUseCaseImpl<PF>
{
    async fn update_profile(
        &self,
        account_id: AccountId,
        country: Option<CountryCode>,
    ) -> Result<(), UpdateProfileError> {
        let new_profile = AccountProfile::new(country);
        match self
            .profile_information_repo
            .insert_profile_information(&account_id, new_profile)
            .await
        {
            Ok(()) => Ok(()),
            Err(RepoError::StorageError(e)) => {
                log::error!(
                    "Failed to retrieve profile information for account {}: {}",
                    account_id,
                    e
                );
                Err(UpdateProfileError::RepositoryError)
            }
        }
    }
}
