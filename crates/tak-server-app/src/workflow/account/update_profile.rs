use std::sync::Arc;

use country_code_enum::CountryCode;

use image::DynamicImage;
use tak_domain::{
    UserId,
    profile::{AccountProfile, ProfilePicture, ProfilePictureVersion},
};

use crate::ports::{
    RepoError, RepoRetrieveError,
    moderation::ModerationRepository,
    profile::{AccountProfileRepository, ProfilePictureRepository},
};

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
    Internal,
}

pub struct UpdateProfileUseCaseImpl<
    PF: AccountProfileRepository,
    M: ModerationRepository,
    PFP: ProfilePictureRepository,
> {
    profile_information_repo: Arc<PF>,
    moderation_repo: Arc<M>,
    profile_picture_repo: Arc<PFP>,
}

impl<PF: AccountProfileRepository, M: ModerationRepository, PFP: ProfilePictureRepository>
    UpdateProfileUseCaseImpl<PF, M, PFP>
{
    pub fn new(
        profile_information_repo: Arc<PF>,
        moderation_repo: Arc<M>,
        profile_picture_repo: Arc<PFP>,
    ) -> Self {
        Self {
            profile_information_repo,
            moderation_repo,
            profile_picture_repo,
        }
    }
}

#[async_trait::async_trait]
impl<
    PF: AccountProfileRepository + Send + Sync + 'static,
    M: ModerationRepository + Send + Sync + 'static,
    PFP: ProfilePictureRepository + Send + Sync + 'static,
> UpdateProfileUseCase for UpdateProfileUseCaseImpl<PF, M, PFP>
{
    async fn update_profile(
        &self,
        user_id: UserId,
        display_name: Option<String>,
        country: Option<CountryCode>,
    ) -> Result<(), UpdateProfileError> {
        match self.moderation_repo.get_moderation(user_id).await {
            Ok(acc) => {
                if acc.is_guest() {
                    return Err(UpdateProfileError::AccountNotFound);
                }
            }
            Err(RepoRetrieveError::NotFound) => {
                return Err(UpdateProfileError::AccountNotFound);
            }
            Err(RepoRetrieveError::StorageError(e)) => {
                tracing::error!(
                    "Failed to retrieve moderation for account {}: {}",
                    user_id,
                    e
                );
                return Err(UpdateProfileError::Internal);
            }
        };
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
                return Err(UpdateProfileError::Internal);
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
                Err(UpdateProfileError::Internal)
            }
        }
    }

    async fn set_profile_picture(
        &self,
        user_id: UserId,
        picture_data: DynamicImage,
    ) -> Result<ProfilePictureVersion, UpdateProfileError> {
        match self.moderation_repo.get_moderation(user_id).await {
            Ok(acc) => {
                if acc.is_guest() {
                    return Err(UpdateProfileError::AccountNotFound);
                }
            }
            Err(RepoRetrieveError::NotFound) => {
                return Err(UpdateProfileError::AccountNotFound);
            }
            Err(RepoRetrieveError::StorageError(e)) => {
                tracing::error!(
                    "Failed to retrieve moderation for account {}: {}",
                    user_id,
                    e
                );
                return Err(UpdateProfileError::Internal);
            }
        };
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
                return Err(UpdateProfileError::Internal);
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
                return Err(UpdateProfileError::Internal);
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
                Err(UpdateProfileError::Internal)
            }
        }
    }
}
