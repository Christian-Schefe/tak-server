use crate::domain::{RepoError, RepoRetrieveError};

#[async_trait::async_trait]
pub trait ProfileInformationRepository {
    async fn upsert_profile_information(
        account_id: String,
        profile_information: ProfileInformation,
    ) -> Result<(), RepoError>;
    async fn get_profile_information(
        account_id: String,
    ) -> Result<ProfileInformation, RepoRetrieveError>;
}

pub struct ProfileInformation {
    display_name: String,
}
