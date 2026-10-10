use std::sync::Arc;

use country_code_enum::CountryCode;

use crate::{
    domain::{
        RepoError, RepoRetrieveError, UserId,
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
        user_id: UserId,
        display_name: Option<String>,
        country: Option<CountryCode>,
    ) -> Result<(), UpdateProfileError>;
    async fn set_profile_picture(
        &self,
        user_id: UserId,
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
    authentication_service: Arc<A>,
    profile_picture_repo: Arc<PFP>,
}

impl<PF: AccountProfileRepository, A: AuthenticationPort, PFP: ProfilePictureRepository>
    UpdateProfileUseCaseImpl<PF, A, PFP>
{
    pub fn new(
        profile_information_repo: Arc<PF>,
        authentication_service: Arc<A>,
        profile_picture_repo: Arc<PFP>,
    ) -> Self {
        Self {
            profile_information_repo,
            authentication_service,
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
        user_id: UserId,
        display_name: Option<String>,
        country: Option<CountryCode>,
    ) -> Result<(), UpdateProfileError> {
        let account = match self.authentication_service.get_account(user_id).await {
            Ok(Some(acc)) => acc,
            Ok(None) => return Err(UpdateProfileError::AccountNotFound),
            Err(_) => return Err(UpdateProfileError::RepositoryError),
        };
        if account.is_guest() {
            return Err(UpdateProfileError::AccountNotFound);
        }
        let mut profile_data = match self
            .profile_information_repo
            .get_profile_information(user_id)
            .await
        {
            Ok(data) => data,
            Err(RepoRetrieveError::NotFound) => AccountProfile::new(),
            Err(RepoRetrieveError::StorageError(e)) => {
                tracing::error!(
                    "Failed to retrieve profile information for account {}: {}",
                    user_id,
                    e
                );
                return Err(UpdateProfileError::RepositoryError);
            }
        };
        profile_data.country = country;
        profile_data.display_name = display_name;
        match self
            .profile_information_repo
            .insert_profile_information(user_id, profile_data)
            .await
        {
            Ok(()) => Ok(()),
            Err(RepoError::StorageError(e)) => {
                tracing::error!(
                    "Failed to update profile information for account {}: {}",
                    user_id,
                    e
                );
                Err(UpdateProfileError::RepositoryError)
            }
        }
    }

    async fn set_profile_picture(
        &self,
        user_id: UserId,
        picture_data: DynamicImage,
    ) -> Result<ProfilePictureVersion, UpdateProfileError> {
        let account = match self.authentication_service.get_account(user_id).await {
            Ok(Some(acc)) => acc,
            Ok(None) => return Err(UpdateProfileError::AccountNotFound),
            Err(_) => return Err(UpdateProfileError::RepositoryError),
        };
        if account.is_guest() {
            return Err(UpdateProfileError::AccountNotFound);
        }
        let profile_data = match self
            .profile_information_repo
            .get_profile_information(user_id)
            .await
        {
            Ok(data) => data,
            Err(RepoRetrieveError::NotFound) => AccountProfile::new(),
            Err(RepoRetrieveError::StorageError(e)) => {
                tracing::error!(
                    "Failed to retrieve profile information for account {}: {}",
                    user_id,
                    e
                );
                return Err(UpdateProfileError::RepositoryError);
            }
        };
        let new_version = profile_data
            .profile_picture_version
            .map(|x| x.increment())
            .unwrap_or(ProfilePictureVersion::initial());
        tracing::info!(
            "Updating profile picture for account {} with new version {}",
            user_id,
            new_version.0
        );
        match self
            .profile_information_repo
            .insert_profile_information(
                user_id,
                AccountProfile {
                    country: profile_data.country,
                    profile_picture_version: Some(new_version),
                    display_name: profile_data.display_name,
                },
            )
            .await
        {
            Ok(()) => (),
            Err(RepoError::StorageError(e)) => {
                tracing::error!(
                    "Failed to update profile information for account {}: {}",
                    user_id,
                    e
                );
                return Err(UpdateProfileError::RepositoryError);
            }
        }
        tracing::info!(
            "Setting new profile picture for account {} with version {}",
            user_id,
            new_version.0
        );
        let prepared_image = ProfilePicture::prepare_image(picture_data);
        match self
            .profile_picture_repo
            .set_profile_picture(user_id, prepared_image)
            .await
        {
            Ok(()) => Ok(new_version),
            Err(RepoError::StorageError(e)) => {
                tracing::error!(
                    "Failed to set profile picture for account {}: {}",
                    user_id,
                    e
                );
                Err(UpdateProfileError::RepositoryError)
            }
        }
    }
}
