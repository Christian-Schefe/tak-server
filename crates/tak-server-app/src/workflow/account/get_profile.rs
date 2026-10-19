use crate::{
    PortsCollection,
    ports::{
        profile::{AccountProfileRepository, ProfilePictureRepository},
        transaction::{TransactionError, TransactionManager},
    },
    workflow::account::AccountProfileView,
};
use std::sync::Arc;
use tak_domain::{UserId, profile::ProfilePicture};

#[async_trait::async_trait]
pub trait GetProfileUseCase {
    async fn get_profile(&self, user_id: UserId) -> Result<AccountProfileView, GetProfileError>;
    async fn get_profile_picture(
        &self,
        user_id: UserId,
    ) -> Result<Option<ProfilePicture>, GetProfileError>;
}

pub enum GetProfileError {
    RepositoryError,
}

pub struct GetProfileUseCaseImpl<Ports: PortsCollection> {
    ports: Arc<Ports>,
}

impl<Ports: PortsCollection> GetProfileUseCaseImpl<Ports> {
    pub fn new(ports: Arc<Ports>) -> Self {
        Self { ports }
    }
}

#[async_trait::async_trait]
impl<Ports: PortsCollection + Send + Sync + 'static> GetProfileUseCase
    for GetProfileUseCaseImpl<Ports>
{
    async fn get_profile(&self, user_id: UserId) -> Result<AccountProfileView, GetProfileError> {
        match self
            .ports
            .transaction_manager()
            .execute(
                self.ports
                    .profile_repository()
                    .get_profile_information(user_id),
            )
            .await
        {
            Ok(Some(profile_information)) => Ok(profile_information.into()),
            Ok(None) => Ok(AccountProfileView {
                display_name: None,
                country: None,
                profile_picture_version: None,
            }),
            Err(TransactionError(e)) => {
                tracing::error!(
                    "Failed to retrieve profile information for account {}: {}",
                    user_id,
                    e
                );
                Err(GetProfileError::RepositoryError)
            }
        }
    }

    async fn get_profile_picture(
        &self,
        user_id: UserId,
    ) -> Result<Option<ProfilePicture>, GetProfileError> {
        match self
            .ports
            .profile_picture_repository()
            .get_profile_picture(user_id)
            .await
        {
            Ok(profile_picture) => Ok(profile_picture),
            Err(TransactionError(e)) => {
                tracing::error!("Failed to retrieve profile picture: {}", e);
                Err(GetProfileError::RepositoryError)
            }
        }
    }
}
