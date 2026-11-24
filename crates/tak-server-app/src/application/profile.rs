use std::sync::Arc;

use better_errors::OneOf;
use country_code_enum::CountryCode;
use image::DynamicImage;
use tak_domain::{
    UserId,
    profile::{Profile, ProfilePicture, ProfilePictureVersion},
};

use crate::{
    PortsCollection,
    errors::{AppResult, InternalError, UserNotFound},
};

pub struct AccountProfileView {
    pub display_name: Option<String>,
    pub country: Option<CountryCode>,
    pub profile_picture_version: Option<ProfilePictureVersion>,
}

impl From<Profile> for AccountProfileView {
    fn from(profile: Profile) -> Self {
        Self {
            display_name: profile.display_name,
            country: profile.country,
            profile_picture_version: profile.profile_picture_version,
        }
    }
}

#[async_trait::async_trait]
pub trait ProfileUseCase {
    async fn get_profile(&self, user_id: UserId)
    -> AppResult<AccountProfileView, (InternalError,)>;
    async fn get_profile_picture(
        &self,
        user_id: UserId,
    ) -> AppResult<Option<ProfilePicture>, (InternalError,)>;
    async fn update_profile(
        &self,
        user_id: UserId,
        display_name: Option<String>,
        country: Option<CountryCode>,
    ) -> AppResult<(), (UserNotFound, InternalError)>;
    async fn set_profile_picture(
        &self,
        user_id: UserId,
        picture_data: DynamicImage,
    ) -> AppResult<ProfilePictureVersion, (UserNotFound, InternalError)>;
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
    async fn get_profile(
        &self,
        user_id: UserId,
    ) -> AppResult<AccountProfileView, (InternalError,)> {
        let ctx = self.ports.transaction_manager().simple();
        match self
            .ports
            .profile_repository()
            .get_profile_information(&ctx, user_id)
            .await?
        {
            Some(profile_information) => Ok(profile_information.into()),
            None => Ok(AccountProfileView {
                display_name: None,
                country: None,
                profile_picture_version: None,
            }),
        }
    }

    async fn get_profile_picture(
        &self,
        user_id: UserId,
    ) -> AppResult<Option<ProfilePicture>, (InternalError,)> {
        self.ports
            .profile_picture_repository()
            .get_profile_picture(user_id)
            .await
    }

    async fn update_profile(
        &self,
        user_id: UserId,
        display_name: Option<String>,
        country: Option<CountryCode>,
    ) -> AppResult<(), (UserNotFound, InternalError)> {
        let ctx = self.ports.transaction_manager().simple();
        let account = self
            .ports
            .moderation_repository()
            .get_moderation(&ctx, user_id)
            .await
            .map_err(OneOf::broaden)?
            .ok_or(OneOf::new(UserNotFound))?;

        if account.is_guest() {
            return Err(OneOf::new(UserNotFound));
        }

        let mut profile_data = match self
            .ports
            .profile_repository()
            .get_profile_information(&ctx, user_id)
            .await
            .map_err(OneOf::broaden)?
        {
            Some(data) => data,
            None => Profile::new(),
        };
        profile_data.country = country;
        profile_data.display_name = display_name;
        self.ports
            .profile_repository()
            .insert_profile_information(&ctx, user_id, profile_data)
            .await
            .map_err(OneOf::broaden)?;
        Ok(())
    }

    async fn set_profile_picture(
        &self,
        user_id: UserId,
        picture_data: DynamicImage,
    ) -> AppResult<ProfilePictureVersion, (UserNotFound, InternalError)> {
        let ctx = self.ports.transaction_manager().simple();
        let account = self
            .ports
            .moderation_repository()
            .get_moderation(&ctx, user_id)
            .await
            .map_err(OneOf::broaden)?
            .ok_or(OneOf::new(UserNotFound))?;
        if account.is_guest() {
            return Err(OneOf::new(UserNotFound));
        }
        let profile_data = self
            .ports
            .profile_repository()
            .get_profile_information(&ctx, user_id)
            .await
            .map_err(OneOf::broaden)?
            .unwrap_or_else(Profile::new);
        let new_version = profile_data
            .profile_picture_version
            .map(|x| x.increment())
            .unwrap_or(ProfilePictureVersion::initial());
        tracing::info!(
            "Updating profile picture for account {} with new version {}",
            user_id,
            new_version.0
        );
        let new_profile_data = Profile {
            country: profile_data.country,
            profile_picture_version: Some(new_version),
            display_name: profile_data.display_name,
        };
        self.ports
            .profile_repository()
            .insert_profile_information(&ctx, user_id, new_profile_data)
            .await
            .map_err(OneOf::broaden)?;
        tracing::info!(
            "Setting new profile picture for account {} with version {}",
            user_id,
            new_version.0
        );
        let prepared_image = ProfilePicture::prepare_image(picture_data);
        self.ports
            .profile_picture_repository()
            .set_profile_picture(user_id, prepared_image)
            .await
            .map_err(OneOf::broaden)?;
        Ok(new_version)
    }
}
