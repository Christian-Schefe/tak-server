use std::sync::Arc;

use crate::{
    domain::{PlayerId, account::AccountRepository},
    ports::authentication::{AuthenticationService, ClientId},
};

pub trait LoginAccountUseCase {
    fn login(&self, username: String, password: String) -> Result<PlayerId, LoginError>;
}

pub enum LoginError {
    InvalidCredentials,
    AccountBanned,
}

pub struct LoginAccountUseCaseImpl<A: AuthenticationService, AR: AccountRepository> {
    authentication_service: Arc<A>,
    account_repository: Arc<AR>,
}

impl<A: AuthenticationService, AR: AccountRepository> LoginAccountUseCaseImpl<A, AR> {
    pub fn new(authentication_service: Arc<A>, account_repository: Arc<AR>) -> Self {
        Self {
            authentication_service,
            account_repository,
        }
    }
}

impl<A: AuthenticationService, AR: AccountRepository> LoginAccountUseCase
    for LoginAccountUseCaseImpl<A, AR>
{
    fn login(&self, username: String, password: String) -> Result<PlayerId, LoginError> {
        let client_id = ClientId::new(&username);
        let client_secret = password;

        if !self
            .authentication_service
            .authenticate(&client_id, &client_secret)
        {
            return Err(LoginError::InvalidCredentials);
        }
        let Some(account) = self.account_repository.get_account_by_client_id(client_id) else {
            return Err(LoginError::InvalidCredentials);
        };
        if account.is_banned {
            return Err(LoginError::AccountBanned);
        }
        Ok(account.player_id)
    }
}
