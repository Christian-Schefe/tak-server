use image::DynamicImage;
use tak_domain::{
    UserId,
    profile::{AccountProfile, ProfilePicture},
};

use crate::errors::{AppResult, InternalError};

#[async_trait::async_trait]
pub trait AccountProfileRepository<RpCtx> {
    async fn insert_profile_information(
        &self,
        ctx: &RpCtx,
        user_id: UserId,
        profile_information: AccountProfile,
    ) -> AppResult<(), (InternalError,)>;
    async fn get_profile_information(
        &self,
        ctx: &RpCtx,
        user_id: UserId,
    ) -> AppResult<Option<AccountProfile>, (InternalError,)>;
}

#[async_trait::async_trait]
pub trait ProfilePictureRepository {
    async fn set_profile_picture(
        &self,
        user_id: UserId,
        image: DynamicImage,
    ) -> AppResult<(), (InternalError,)>;
    async fn get_profile_picture(
        &self,
        user_id: UserId,
    ) -> AppResult<Option<ProfilePicture>, (InternalError,)>;
}
