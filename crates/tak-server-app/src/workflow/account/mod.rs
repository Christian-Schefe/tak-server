use country_code_enum::CountryCode;

use crate::domain::profile::{AccountProfile, ProfilePictureVersion};

pub mod get_online;
pub mod get_profile;
pub mod get_snapshot;
pub mod moderate;
pub mod set_online;
pub mod update_profile;

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
