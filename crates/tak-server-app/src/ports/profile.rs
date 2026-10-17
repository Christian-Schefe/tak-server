use image::DynamicImage;
use tak_domain::{
    UserId,
    profile::{AccountProfile, ProfilePicture},
};

use crate::ports::{RepoError, RepoRetrieveError};

#[async_trait::async_trait]
pub trait AccountProfileRepository {
    async fn insert_profile_information(
        &self,
        user_id: UserId,
        profile_information: AccountProfile,
    ) -> Result<(), RepoError>;
    async fn get_profile_information(
        &self,
        user_id: UserId,
    ) -> Result<AccountProfile, RepoRetrieveError>;
}

#[async_trait::async_trait]
pub trait ProfilePictureRepository {
    async fn set_profile_picture(
        &self,
        user_id: UserId,
        image: DynamicImage,
    ) -> Result<(), RepoError>;
    async fn get_profile_picture(
        &self,
        user_id: UserId,
    ) -> Result<ProfilePicture, RepoRetrieveError>;
}
