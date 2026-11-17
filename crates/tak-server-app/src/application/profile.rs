use std::sync::Arc;

use country_code_enum::CountryCode;
use image::DynamicImage;
use tak_domain::{
    UserId,
    profile::{AccountProfile, ProfilePicture, ProfilePictureVersion},
};

use crate::{
    PortsCollection,
    ports::{
        moderation::ModerationRepository,
        profile::{AccountProfileRepository, ProfilePictureRepository},
        transaction::{RepoError, TransactionManager},
    },
};

pub struct AccountProfileView {
    pub display_name: Option<String>,
    pub country: Option<CountryCode>,
    pub profile_picture_version: Option<ProfilePictureVersion>,
}

impl From<AccountProfile> for AccountProfileView {
    fn from(profile: AccountProfile) -> Self {
        Self {
            display_name: profile.display_name,
            country: profile.country,
            profile_picture_version: profile.profile_picture_version,
        }
    }
}

pub enum UpdateProfileError {
    AccountNotFound,
    Internal,
}

pub enum GetProfileError {
    RepositoryError,
}

#[async_trait::async_trait]
pub trait ProfileUseCase {
    async fn get_profile(&self, user_id: UserId) -> Result<AccountProfileView, GetProfileError>;
    async fn get_profile_picture(
        &self,
        user_id: UserId,
    ) -> Result<Option<ProfilePicture>, GetProfileError>;
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

pub struct ProfileUseCaseImpl<Ports: PortsCollection> {
    ports: Arc<Ports>,
}

impl<Ports: PortsCollection> ProfileUseCaseImpl<Ports> {
    pub fn new(ports: Arc<Ports>) -> Self {
        Self { ports }
    }
}

#[async_trait::async_trait]
impl<Ports: PortsCollection + Send + Sync + 'static> ProfileUseCase for ProfileUseCaseImpl<Ports> {
    async fn get_profile(&self, user_id: UserId) -> Result<AccountProfileView, GetProfileError> {
        let ctx = self.ports.transaction_manager().simple();
        match self
            .ports
            .profile_repository()
            .get_profile_information(&ctx, user_id)
            .await
        {
            Ok(Some(profile_information)) => Ok(profile_information.into()),
            Ok(None) => Ok(AccountProfileView {
                display_name: None,
                country: None,
                profile_picture_version: None,
            }),
            Err(RepoError(e)) => {
                tracing::error!(
                    "Failed to retrieve profile information for account {}: {}",
                    user_id,
                    e
                );
                Err(GetProfileError::RepositoryError)
            }
        }
    }

    async fn get_profile_picture(
        &self,
        user_id: UserId,
    ) -> Result<Option<ProfilePicture>, GetProfileError> {
        match self
            .ports
            .profile_picture_repository()
            .get_profile_picture(user_id)
            .await
        {
            Ok(profile_picture) => Ok(profile_picture),
            Err(RepoError(e)) => {
                tracing::error!("Failed to retrieve profile picture: {}", e);
                Err(GetProfileError::RepositoryError)
            }
        }
    }

    async fn update_profile(
        &self,
        user_id: UserId,
        display_name: Option<String>,
        country: Option<CountryCode>,
    ) -> Result<(), UpdateProfileError> {
        let ctx = self.ports.transaction_manager().simple();
        match self
            .ports
            .moderation_repository()
            .get_moderation(&ctx, user_id)
            .await
        {
            Ok(Some(acc)) => {
                if acc.is_guest() {
                    return Err(UpdateProfileError::AccountNotFound);
                }
            }
            Ok(None) => {
                return Err(UpdateProfileError::AccountNotFound);
            }
            Err(RepoError(e)) => {
                tracing::error!(
                    "Failed to retrieve moderation for account {}: {}",
                    user_id,
                    e
                );
                return Err(UpdateProfileError::Internal);
            }
        }

        let mut profile_data = match self
            .ports
            .profile_repository()
            .get_profile_information(&ctx, user_id)
            .await
        {
            Ok(Some(data)) => data,
            Ok(None) => AccountProfile::new(),
            Err(RepoError(e)) => {
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
            .ports
            .profile_repository()
            .insert_profile_information(&ctx, user_id, profile_data)
            .await
        {
            Ok(()) => Ok(()),
            Err(RepoError(e)) => {
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
        let ctx = self.ports.transaction_manager().simple();
        match self
            .ports
            .moderation_repository()
            .get_moderation(&ctx, user_id)
            .await
        {
            Ok(Some(acc)) => {
                if acc.is_guest() {
                    return Err(UpdateProfileError::AccountNotFound);
                }
            }
            Ok(None) => {
                return Err(UpdateProfileError::AccountNotFound);
            }
            Err(RepoError(e)) => {
                tracing::error!(
                    "Failed to retrieve moderation for account {}: {}",
                    user_id,
                    e
                );
                return Err(UpdateProfileError::Internal);
            }
        };
        let profile_data = match self
            .ports
            .profile_repository()
            .get_profile_information(&ctx, user_id)
            .await
        {
            Ok(Some(data)) => data,
            Ok(None) => AccountProfile::new(),
            Err(RepoError(e)) => {
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
        let new_profile_data = AccountProfile {
            country: profile_data.country,
            profile_picture_version: Some(new_version),
            display_name: profile_data.display_name,
        };
        match self
            .ports
            .profile_repository()
            .insert_profile_information(&ctx, user_id, new_profile_data)
            .await
        {
            Ok(()) => (),
            Err(RepoError(e)) => {
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
            .ports
            .profile_picture_repository()
            .set_profile_picture(user_id, prepared_image)
            .await
        {
            Ok(()) => Ok(new_version),
            Err(RepoError(e)) => {
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
