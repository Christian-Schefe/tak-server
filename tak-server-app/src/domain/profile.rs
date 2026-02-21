use country_code_enum::CountryCode;
use image::DynamicImage;

use crate::domain::{AccountId, RepoError, RepoRetrieveError};

#[async_trait::async_trait]
pub trait AccountProfileRepository {
    async fn insert_profile_information(
        &self,
        account_id: &AccountId,
        profile_information: AccountProfile,
    ) -> Result<(), RepoError>;
    async fn get_profile_information(
        &self,
        account_id: &AccountId,
    ) -> Result<AccountProfile, RepoRetrieveError>;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ProfilePictureVersion(pub u64);

impl ProfilePictureVersion {
    pub fn initial() -> Self {
        Self(0)
    }
    pub fn increment(&self) -> Self {
        Self(self.0 + 1)
    }
}

#[derive(Clone, Debug)]
pub struct AccountProfile {
    pub country: Option<CountryCode>,
    pub profile_picture_version: ProfilePictureVersion,
}

impl AccountProfile {
    pub fn new(
        country: Option<CountryCode>,
        profile_picture_version: ProfilePictureVersion,
    ) -> Self {
        Self {
            country,
            profile_picture_version,
        }
    }
}

#[async_trait::async_trait]
pub trait ProfilePictureRepository {
    async fn set_profile_picture(
        &self,
        account_id: &AccountId,
        image: DynamicImage,
    ) -> Result<(), RepoError>;
    async fn get_profile_picture(
        &self,
        account_id: &AccountId,
    ) -> Result<ProfilePicture, RepoRetrieveError>;
    async fn get_default_profile_picture(&self) -> Result<ProfilePicture, RepoRetrieveError>;
}

pub struct ProfilePicture {
    pub stream: ProfilePictureStream,
    pub content_type: ProfilePictureFileType,
}

impl ProfilePicture {
    pub fn new(stream: ProfilePictureStream, content_type: ProfilePictureFileType) -> Self {
        Self {
            stream,
            content_type,
        }
    }
}

pub enum ProfilePictureFileType {
    WebP,
}

pub type ProfilePictureStream =
    Box<dyn futures::Stream<Item = Result<bytes::Bytes, std::io::Error>> + Send + Unpin>;
