use country_code_enum::CountryCode;

use crate::domain::profile::AccountProfile;

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
}

impl From<AccountProfile> for AccountProfileView {
    fn from(profile: AccountProfile) -> Self {
        Self {
            country: profile.country,
        }
    }
}
