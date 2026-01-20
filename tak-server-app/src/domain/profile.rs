use country_code_enum::CountryCode;

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

#[derive(Clone, Debug)]
pub struct AccountProfile {
    pub country: Option<CountryCode>,
}

impl AccountProfile {
    pub fn new(country: Option<CountryCode>) -> Self {
        Self { country }
    }
}
