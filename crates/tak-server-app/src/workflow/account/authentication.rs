use std::{sync::Arc, time::Duration};

use crate::{
    PortsCollection,
    ports::{
        authentication::{AccountPort, RefreshToken},
        email::EmailPort,
        moderation::ModerationRepository,
        transaction::{TransactionError, TransactionManager},
    },
};
use tak_domain::{
    UserId,
    moderation::{Moderation, UserType},
};
use url::Url;

pub enum RecoverError {
    AccountNotFound,
    EmailNotSet,
    Internal,
}

pub enum ResetError {
    AccountNotFound,
    InvalidToken,
    Internal,
}

pub enum LoginError {
    InvalidCredentials,
    Internal,
}

pub enum RegisterError {
    Internal,
}

pub enum RefreshError {
    InvalidToken,
    Internal,
}

#[derive(Clone, Debug)]
pub struct AuthenticatedUser {
    pub user_id: UserId,
    pub refresh_token: RefreshToken,
}

#[async_trait::async_trait]
pub trait AuthenticationUseCase {
    async fn login(
        &self,
        identifier: String,
        password: String,
    ) -> Result<AuthenticatedUser, LoginError>;
    async fn register(
        &self,
        email: String,
        username: String,
        password: String,
    ) -> Result<AuthenticatedUser, RegisterError>;
    async fn recover(&self, identifier: String) -> Result<(), RecoverError>;
    async fn reset(
        &self,
        identifier: String,
        token: String,
        new_password: String,
    ) -> Result<(), ResetError>;
    async fn create_guest_account(&self) -> Result<AuthenticatedUser, RegisterError>;
    async fn refresh(
        &self,
        user_id: UserId,
        refresh_token: String,
    ) -> Result<AuthenticatedUser, RefreshError>;
}

pub struct AuthenticationUseCaseImpl<Ports: PortsCollection, A: AccountPort, E: EmailPort> {
    ports: Arc<Ports>,
    account_service: Arc<A>,
    email_service: Arc<E>,
}
impl<Ports: PortsCollection, A: AccountPort, E: EmailPort> AuthenticationUseCaseImpl<Ports, A, E> {
    pub fn new(ports: Arc<Ports>, account_service: Arc<A>, email_service: Arc<E>) -> Self {
        Self {
            ports,
            account_service,
            email_service,
        }
    }
}

const REFRESH_TOKEN_DURATION: Duration = Duration::from_secs(60 * 60 * 24 * 7);

#[async_trait::async_trait]
impl<
    Ports: PortsCollection + Send + Sync + 'static,
    A: AccountPort + Send + Sync + 'static,
    E: EmailPort + Send + Sync + 'static,
> AuthenticationUseCase for AuthenticationUseCaseImpl<Ports, A, E>
{
    async fn login(
        &self,
        identifier: String,
        password: String,
    ) -> Result<AuthenticatedUser, LoginError> {
        match self
            .account_service
            .login_account(&identifier, &password)
            .await
        {
            Ok(Some(user_id)) => Ok(AuthenticatedUser {
                user_id,
                refresh_token: self
                    .account_service
                    .generate_refresh_token(user_id.clone(), REFRESH_TOKEN_DURATION)
                    .map_err(|e| {
                        tracing::error!("Failed to generate refresh token: {:?}", e);
                        LoginError::Internal
                    })?,
            }),
            Ok(None) => Err(LoginError::InvalidCredentials),
            Err(e) => {
                tracing::error!("Failed to login account: {:?}", e);
                Err(LoginError::Internal)
            }
        }
    }

    async fn register(
        &self,
        email: String,
        username: String,
        password: String,
    ) -> Result<AuthenticatedUser, RegisterError> {
        let user_id = match self
            .account_service
            .register_account(&email, &username, &password)
            .await
        {
            Ok(user_id) => user_id,
            Err(e) => {
                tracing::error!("Failed to register account: {:?}", e);
                return Err(RegisterError::Internal);
            }
        };
        let mut moderation = Moderation::default();
        moderation.user_type = UserType::Guest;

        match self
            .ports
            .transaction_manager()
            .execute(
                self.ports
                    .moderation_repository()
                    .set_moderation(user_id, moderation),
            )
            .await
        {
            Ok(()) => (),
            Err(TransactionError(e)) => {
                tracing::error!("Failed to set moderation for guest account: {:?}", e);
                return Err(RegisterError::Internal);
            }
        }
        let refresh_token = self
            .account_service
            .generate_refresh_token(user_id.clone(), REFRESH_TOKEN_DURATION)
            .map_err(|e| {
                tracing::error!("Failed to generate refresh token: {:?}", e);
                RegisterError::Internal
            })?;

        Ok(AuthenticatedUser {
            user_id,
            refresh_token,
        })
    }

    async fn recover(&self, identifier: String) -> Result<(), RecoverError> {
        let user_id = match self.account_service.find_account(&identifier).await {
            Ok(Some(user_id)) => user_id,
            Ok(None) => {
                return Err(RecoverError::AccountNotFound);
            }
            Err(e) => {
                tracing::error!("Error finding account: {:?}", e);
                return Err(RecoverError::Internal);
            }
        };

        let account_info = match self.account_service.get_account_info(user_id).await {
            Ok(Some(account_info)) => account_info,
            Ok(None) => {
                return Err(RecoverError::AccountNotFound);
            }
            Err(e) => {
                tracing::error!("Error retrieving account info: {:?}", e);
                return Err(RecoverError::Internal);
            }
        };

        let Some(email) = &account_info.email else {
            return Err(RecoverError::EmailNotSet);
        };
        let token = self
            .account_service
            .generate_reset_token(user_id, Duration::from_mins(30))
            .map_err(|_| {
                tracing::error!("Failed to generate reset token");
                RecoverError::Internal
            })?;

        let site_address =
            std::env::var("SITE_ADDRESS").expect("SITE_ADDRESS environment variable must be set");

        let mut base_url = Url::parse(&format!("https://{}/reset", site_address)).unwrap();
        base_url.query_pairs_mut().append_pair("token", &token);

        let body = format!(
            "Hello {},\n\nYou requested a password reset. Please click the link below to login and reset your password:\n\n{}\n\nThe link will expire in 30 minutes.\n\nIf you did not request this, please ignore this email.",
            account_info.username, base_url
        );

        match self
            .email_service
            .send_email(email, "Password Reset", &body)
        {
            Ok(()) => Ok(()),
            Err(e) => {
                tracing::error!("Failed to send password reset email: {:?}", e);
                Err(RecoverError::Internal)
            }
        }
    }

    async fn reset(
        &self,
        identifier: String,
        token: String,
        new_password: String,
    ) -> Result<(), ResetError> {
        let user_id = match self.account_service.find_account(&identifier).await {
            Ok(Some(user_id)) => user_id,
            Ok(None) => {
                return Err(ResetError::AccountNotFound);
            }
            Err(e) => {
                tracing::error!("Error finding account: {:?}", e);
                return Err(ResetError::Internal);
            }
        };
        match self
            .account_service
            .reset_account_password(user_id, &token, &new_password)
            .await
        {
            Ok(Some(())) => Ok(()),
            Ok(None) => Err(ResetError::InvalidToken),
            Err(e) => {
                tracing::error!("Error resetting account password: {:?}", e);
                Err(ResetError::Internal)
            }
        }
    }

    async fn create_guest_account(&self) -> Result<AuthenticatedUser, RegisterError> {
        let user_id = match self.account_service.create_guest_account().await {
            Ok(user_id) => user_id,
            Err(e) => {
                tracing::error!("Failed to create guest account: {:?}", e);
                return Err(RegisterError::Internal);
            }
        };

        let mut moderation = Moderation::default();
        moderation.user_type = UserType::Guest;

        match self
            .ports
            .transaction_manager()
            .execute(
                self.ports
                    .moderation_repository()
                    .set_moderation(user_id, moderation),
            )
            .await
        {
            Ok(()) => (),
            Err(TransactionError(e)) => {
                tracing::error!("Failed to set moderation for guest account: {:?}", e);
                return Err(RegisterError::Internal);
            }
        }

        let refresh_token = self
            .account_service
            .generate_refresh_token(user_id, REFRESH_TOKEN_DURATION)
            .map_err(|e| {
                tracing::error!("Failed to generate refresh token: {:?}", e);
                RegisterError::Internal
            })?;

        Ok(AuthenticatedUser {
            user_id: user_id,
            refresh_token,
        })
    }

    async fn refresh(
        &self,
        user_id: UserId,
        refresh_token: String,
    ) -> Result<AuthenticatedUser, RefreshError> {
        match self.account_service.rotate_refresh_token(
            user_id,
            &refresh_token,
            REFRESH_TOKEN_DURATION,
        ) {
            Ok(Some(new_refresh_token)) => Ok(AuthenticatedUser {
                user_id,
                refresh_token: new_refresh_token,
            }),
            Ok(None) => Err(RefreshError::InvalidToken),
            Err(e) => {
                tracing::error!("Error rotating refresh token: {:?}", e);
                Err(RefreshError::Internal)
            }
        }
    }
}
