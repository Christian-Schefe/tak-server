use std::sync::Arc;

use crate::{
    domain::{
        AccountId, RepoRetrieveError,
        profile::{AccountProfileRepository, ProfilePicture, ProfilePictureRepository},
    },
    workflow::account::AccountProfileView,
};

#[async_trait::async_trait]
pub trait GetProfileUseCase {
    async fn get_profile(
        &self,
        account_id: &AccountId,
    ) -> Result<AccountProfileView, GetProfileError>;
    async fn get_profile_picture(
        &self,
        account_id: &AccountId,
    ) -> Result<Option<ProfilePicture>, GetProfileError>;
}

pub enum GetProfileError {
    RepositoryError,
}

pub struct GetProfileUseCaseImpl<PF: AccountProfileRepository, PFP: ProfilePictureRepository> {
    profile_information_repo: Arc<PF>,
    profile_picture_repo: Arc<PFP>,
}

impl<PF: AccountProfileRepository, PFP: ProfilePictureRepository> GetProfileUseCaseImpl<PF, PFP> {
    pub fn new(profile_information_repo: Arc<PF>, profile_picture_repo: Arc<PFP>) -> Self {
        Self {
            profile_information_repo,
            profile_picture_repo,
        }
    }
}

#[async_trait::async_trait]
impl<
    PF: AccountProfileRepository + Send + Sync + 'static,
    PFP: ProfilePictureRepository + Send + Sync + 'static,
> GetProfileUseCase for GetProfileUseCaseImpl<PF, PFP>
{
    async fn get_profile(
        &self,
        account_id: &AccountId,
    ) -> Result<AccountProfileView, GetProfileError> {
        match self
            .profile_information_repo
            .get_profile_information(&account_id)
            .await
        {
            Ok(profile_information) => Ok(profile_information.into()),
            Err(RepoRetrieveError::NotFound) => Ok(AccountProfileView {
                country: None,
                profile_picture_version: None,
            }),
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

    async fn get_profile_picture(
        &self,
        account_id: &AccountId,
    ) -> Result<Option<ProfilePicture>, GetProfileError> {
        match self
            .profile_picture_repo
            .get_profile_picture(account_id)
            .await
        {
            Ok(profile_picture) => Ok(Some(profile_picture)),
            Err(RepoRetrieveError::NotFound) => Ok(None),
            Err(RepoRetrieveError::StorageError(e)) => {
                log::error!("Failed to retrieve profile picture: {}", e);
                Err(GetProfileError::RepositoryError)
            }
        }
    }
}
