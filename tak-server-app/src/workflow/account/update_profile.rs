use std::sync::Arc;

use country_code_enum::CountryCode;

use crate::{
    domain::{
        AccountId, RepoError,
        profile::{AccountProfile, AccountProfileRepository},
    },
    ports::authentication::AuthenticationPort,
};

#[async_trait::async_trait]
pub trait UpdateProfileUseCase {
    async fn update_profile(
        &self,
        account_id: AccountId,
        country: Option<CountryCode>,
    ) -> Result<(), UpdateProfileError>;
}

pub enum UpdateProfileError {
    ProfileNotFound,
    RepositoryError,
}

pub struct UpdateProfileUseCaseImpl<PF: AccountProfileRepository, A: AuthenticationPort> {
    profile_information_repo: Arc<PF>,
    authentication_port: Arc<A>,
}

impl<PF: AccountProfileRepository, A: AuthenticationPort> UpdateProfileUseCaseImpl<PF, A> {
    pub fn new(profile_information_repo: Arc<PF>, authentication_port: Arc<A>) -> Self {
        Self {
            profile_information_repo,
            authentication_port,
        }
    }
}

#[async_trait::async_trait]
impl<
    PF: AccountProfileRepository + Send + Sync + 'static,
    A: AuthenticationPort + Send + Sync + 'static,
> UpdateProfileUseCase for UpdateProfileUseCaseImpl<PF, A>
{
    async fn update_profile(
        &self,
        account_id: AccountId,
        country: Option<CountryCode>,
    ) -> Result<(), UpdateProfileError> {
        let account = match self.authentication_port.get_account(&account_id).await {
            Some(acc) => acc,
            None => return Err(UpdateProfileError::ProfileNotFound),
        };
        if account.is_guest() {
            return Err(UpdateProfileError::ProfileNotFound);
        }
        let new_profile = AccountProfile::new(country);
        match self
            .profile_information_repo
            .insert_profile_information(&account_id, new_profile)
            .await
        {
            Ok(()) => Ok(()),
            Err(RepoError::StorageError(e)) => {
                log::error!(
                    "Failed to retrieve profile information for account {}: {}",
                    account_id,
                    e
                );
                Err(UpdateProfileError::RepositoryError)
            }
        }
    }
}
