use std::{sync::Arc, time::Duration};

use crate::{
    PortsCollection,
    errors::{
        AppResult, EmailTaken, InternalError, InvalidCredentials, InvalidEmailAddress, UserNotFound,
    },
    ports::{
        authentication::{AccountPort, RefreshToken},
        email::EmailPort,
        moderation::ModerationRepository,
        transaction::TransactionManager,
    },
};
use better_errors::OneOf;
use tak_domain::{
    UserId,
    moderation::{Moderation, UserType},
};
use url::Url;

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
    ) -> AppResult<AuthenticatedUser, (InvalidCredentials, InternalError)>;
    async fn register(
        &self,
        email: String,
        username: String,
        password: String,
    ) -> AppResult<AuthenticatedUser, (EmailTaken, InternalError)>;
    async fn recover(
        &self,
        identifier: String,
    ) -> AppResult<(), (UserNotFound, InvalidEmailAddress, InternalError)>;
    async fn reset(
        &self,
        identifier: String,
        token: String,
        new_password: String,
    ) -> AppResult<(), (InvalidCredentials, InternalError)>;
    async fn create_guest_account(&self) -> AppResult<AuthenticatedUser, (InternalError,)>;
    async fn refresh(
        &self,
        user_id: UserId,
        refresh_token: String,
    ) -> AppResult<AuthenticatedUser, (InvalidCredentials, InternalError)>;
    async fn logout(&self, user_id: UserId) -> AppResult<(), (InternalError,)>;
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
    ) -> AppResult<AuthenticatedUser, (InvalidCredentials, InternalError)> {
        let user_id = self
            .account_service
            .login_account(&identifier, &password)
            .await?;
        Ok(AuthenticatedUser {
            user_id,
            refresh_token: self
                .account_service
                .generate_refresh_token(user_id.clone(), REFRESH_TOKEN_DURATION)
                .map_err(OneOf::broaden)?,
        })
    }

    async fn register(
        &self,
        email: String,
        username: String,
        password: String,
    ) -> AppResult<AuthenticatedUser, (EmailTaken, InternalError)> {
        let user_id = self
            .account_service
            .register_account(&email, &username, &password)
            .await?;

        let mut moderation = Moderation::default();
        moderation.user_type = UserType::Guest;

        let ctx = self.ports.transaction_manager().simple();
        self.ports
            .moderation_repository()
            .set_moderation(&ctx, user_id, moderation)
            .await
            .map_err(OneOf::broaden)?;
        let refresh_token = self
            .account_service
            .generate_refresh_token(user_id.clone(), REFRESH_TOKEN_DURATION)
            .map_err(OneOf::broaden)?;

        Ok(AuthenticatedUser {
            user_id,
            refresh_token,
        })
    }

    async fn recover(
        &self,
        identifier: String,
    ) -> AppResult<(), (UserNotFound, InvalidEmailAddress, InternalError)> {
        let Some(user_id) = self
            .account_service
            .find_account(&identifier)
            .await
            .map_err(OneOf::broaden)?
        else {
            return Err(OneOf::new(UserNotFound));
        };

        let Some(account_info) = self
            .account_service
            .get_account_info(user_id)
            .await
            .map_err(OneOf::broaden)?
        else {
            return Err(OneOf::new(UserNotFound));
        };

        let Some(email) = &account_info.email else {
            return Err(OneOf::new(InvalidEmailAddress));
        };
        let token = self
            .account_service
            .generate_reset_token(user_id, Duration::from_mins(30))
            .map_err(OneOf::broaden)?;

        let site_address =
            std::env::var("SITE_ADDRESS").expect("SITE_ADDRESS environment variable must be set");

        let mut base_url = Url::parse(&format!("https://{}/reset", site_address)).unwrap();
        base_url.query_pairs_mut().append_pair("token", &token);

        let body = format!(
            "Hello {},\n\nYou requested a password reset. Please click the link below to login and reset your password:\n\n{}\n\nThe link will expire in 30 minutes.\n\nIf you did not request this, please ignore this email.",
            account_info.username, base_url
        );

        self.email_service
            .send_email(email, "Password Reset", &body)
            .map_err(OneOf::broaden)?;
        Ok(())
    }

    async fn reset(
        &self,
        identifier: String,
        token: String,
        new_password: String,
    ) -> AppResult<(), (InvalidCredentials, InternalError)> {
        let Some(user_id) = self
            .account_service
            .find_account(&identifier)
            .await
            .map_err(OneOf::broaden)?
        else {
            return Err(OneOf::new(InvalidCredentials));
        };
        self.account_service
            .reset_account_password(user_id, &token, &new_password)
            .await?;
        Ok(())
    }

    async fn create_guest_account(&self) -> AppResult<AuthenticatedUser, (InternalError,)> {
        let user_id = self.account_service.create_guest_account().await?;

        let mut moderation = Moderation::default();
        moderation.user_type = UserType::Guest;

        let ctx = self.ports.transaction_manager().simple();
        self.ports
            .moderation_repository()
            .set_moderation(&ctx, user_id, moderation)
            .await?;

        let refresh_token = self
            .account_service
            .generate_refresh_token(user_id, REFRESH_TOKEN_DURATION)?;

        Ok(AuthenticatedUser {
            user_id: user_id,
            refresh_token,
        })
    }

    async fn refresh(
        &self,
        user_id: UserId,
        refresh_token: String,
    ) -> AppResult<AuthenticatedUser, (InvalidCredentials, InternalError)> {
        let new_token = self.account_service.rotate_refresh_token(
            user_id,
            &refresh_token,
            REFRESH_TOKEN_DURATION,
        )?;

        Ok(AuthenticatedUser {
            user_id,
            refresh_token: new_token,
        })
    }

    async fn logout(&self, user_id: UserId) -> AppResult<(), (InternalError,)> {
        self.account_service.invalidate_refresh_token(user_id)
    }
}
