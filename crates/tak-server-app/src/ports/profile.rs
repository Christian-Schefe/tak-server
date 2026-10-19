use image::DynamicImage;
use tak_domain::{
    UserId,
    profile::{AccountProfile, ProfilePicture},
};

use crate::ports::transaction::{TransactionError, TransactionItem};

pub trait AccountProfileRepository<TrCtx> {
    fn insert_profile_information(
        &self,
        user_id: UserId,
        profile_information: AccountProfile,
    ) -> TransactionItem<TrCtx, Result<(), TransactionError>>;
    fn get_profile_information(
        &self,
        user_id: UserId,
    ) -> TransactionItem<TrCtx, Result<Option<AccountProfile>, TransactionError>>;
}

#[async_trait::async_trait]
pub trait ProfilePictureRepository {
    async fn set_profile_picture(
        &self,
        user_id: UserId,
        image: DynamicImage,
    ) -> Result<(), TransactionError>;
    async fn get_profile_picture(
        &self,
        user_id: UserId,
    ) -> Result<Option<ProfilePicture>, TransactionError>;
}
