use country_code_enum::CountryCode;

use crate::domain::profile::{AccountProfile, ProfilePictureVersion};

pub mod cleanup_guests;
pub mod get_account;
pub mod get_online;
pub mod get_profile;
pub mod get_snapshot;
pub mod moderate;
pub mod remove_account;
pub mod set_online;
pub mod update_profile;

pub struct AccountProfileView {
    pub country: Option<CountryCode>,
    pub profile_picture_version: ProfilePictureVersion,
}

impl From<AccountProfile> for AccountProfileView {
    fn from(profile: AccountProfile) -> Self {
        Self {
            country: profile.country,
            profile_picture_version: profile.profile_picture_version,
        }
    }
}
