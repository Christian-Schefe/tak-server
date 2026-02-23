use std::sync::Arc;

use country_code_enum::CountryCode;

use crate::{
    domain::{
        AccountId, RepoError, RepoRetrieveError,
        profile::{
            AccountProfile, AccountProfileRepository, ProfilePicture, ProfilePictureRepository,
            ProfilePictureVersion,
        },
    },
    ports::authentication::AuthenticationPort,
};
use image::DynamicImage;

#[async_trait::async_trait]
pub trait UpdateProfileUseCase {
    async fn update_profile(
        &self,
        account_id: &AccountId,
        country: Option<CountryCode>,
    ) -> Result<(), UpdateProfileError>;
    async fn set_profile_picture(
        &self,
        account_id: &AccountId,
        picture_data: DynamicImage,
    ) -> Result<ProfilePictureVersion, UpdateProfileError>;
}

pub enum UpdateProfileError {
    AccountNotFound,
    RepositoryError,
}

pub struct UpdateProfileUseCaseImpl<
    PF: AccountProfileRepository,
    A: AuthenticationPort,
    PFP: ProfilePictureRepository,
> {
    profile_information_repo: Arc<PF>,
    authentication_port: Arc<A>,
    profile_picture_repo: Arc<PFP>,
}

impl<PF: AccountProfileRepository, A: AuthenticationPort, PFP: ProfilePictureRepository>
    UpdateProfileUseCaseImpl<PF, A, PFP>
{
    pub fn new(
        profile_information_repo: Arc<PF>,
        authentication_port: Arc<A>,
        profile_picture_repo: Arc<PFP>,
    ) -> Self {
        Self {
            profile_information_repo,
            authentication_port,
            profile_picture_repo,
        }
    }
}

#[async_trait::async_trait]
impl<
    PF: AccountProfileRepository + Send + Sync + 'static,
    A: AuthenticationPort + Send + Sync + 'static,
    PFP: ProfilePictureRepository + Send + Sync + 'static,
> UpdateProfileUseCase for UpdateProfileUseCaseImpl<PF, A, PFP>
{
    async fn update_profile(
        &self,
        account_id: &AccountId,
        country: Option<CountryCode>,
    ) -> Result<(), UpdateProfileError> {
        let account = match self.authentication_port.get_account(account_id).await {
            Some(acc) => acc,
            None => return Err(UpdateProfileError::AccountNotFound),
        };
        if account.is_guest() {
            return Err(UpdateProfileError::AccountNotFound);
        }
        let mut profile_data = match self
            .profile_information_repo
            .get_profile_information(account_id)
            .await
        {
            Ok(data) => data,
            Err(RepoRetrieveError::NotFound) => AccountProfile::new(None, None),
            Err(RepoRetrieveError::StorageError(e)) => {
                log::error!(
                    "Failed to retrieve profile information for account {}: {}",
                    account_id,
                    e
                );
                return Err(UpdateProfileError::RepositoryError);
            }
        };
        profile_data.country = country;
        match self
            .profile_information_repo
            .insert_profile_information(account_id, profile_data)
            .await
        {
            Ok(()) => Ok(()),
            Err(RepoError::StorageError(e)) => {
                log::error!(
                    "Failed to update profile information for account {}: {}",
                    account_id,
                    e
                );
                Err(UpdateProfileError::RepositoryError)
            }
        }
    }

    async fn set_profile_picture(
        &self,
        account_id: &AccountId,
        picture_data: DynamicImage,
    ) -> Result<ProfilePictureVersion, UpdateProfileError> {
        let account = match self.authentication_port.get_account(account_id).await {
            Some(acc) => acc,
            None => return Err(UpdateProfileError::AccountNotFound),
        };
        if account.is_guest() {
            return Err(UpdateProfileError::AccountNotFound);
        }
        let profile_data = match self
            .profile_information_repo
            .get_profile_information(account_id)
            .await
        {
            Ok(data) => data,
            Err(RepoRetrieveError::NotFound) => AccountProfile::new(None, None),
            Err(RepoRetrieveError::StorageError(e)) => {
                log::error!(
                    "Failed to retrieve profile information for account {}: {}",
                    account_id,
                    e
                );
                return Err(UpdateProfileError::RepositoryError);
            }
        };
        let new_version = profile_data
            .profile_picture_version
            .map(|x| x.increment())
            .unwrap_or(ProfilePictureVersion::initial());
        log::info!(
            "Updating profile picture for account {} with new version {}",
            account_id,
            new_version.0
        );
        match self
            .profile_information_repo
            .insert_profile_information(
                account_id,
                AccountProfile {
                    country: profile_data.country,
                    profile_picture_version: Some(new_version),
                },
            )
            .await
        {
            Ok(()) => (),
            Err(RepoError::StorageError(e)) => {
                log::error!(
                    "Failed to update profile information for account {}: {}",
                    account_id,
                    e
                );
                return Err(UpdateProfileError::RepositoryError);
            }
        }
        log::info!(
            "Setting new profile picture for account {} with version {}",
            account_id,
            new_version.0
        );
        let prepared_image = ProfilePicture::prepare_image(picture_data);
        match self
            .profile_picture_repo
            .set_profile_picture(account_id, prepared_image)
            .await
        {
            Ok(()) => Ok(new_version),
            Err(RepoError::StorageError(e)) => {
                log::error!(
                    "Failed to set profile picture for account {}: {}",
                    account_id,
                    e
                );
                Err(UpdateProfileError::RepositoryError)
            }
        }
    }
}
