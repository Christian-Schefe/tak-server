use std::sync::Arc;

use passwords::PasswordGenerator;

use crate::{
    domain::account::{AccountFactory, AccountRepository, CreateAccountRepoError},
    ports::{
        authentication::{AuthRegisterError, AuthenticationService, ClientId},
        contact::{ContactRepository, StoreEmailError},
    },
};

pub trait RegisterAccountUseCase {
    fn register_account(&self, username: String, email: String) -> Result<(), CreateAccountError>;
}

pub enum CreateAccountError {
    UsernameTaken,
    InvalidUsername,
    InvalidEmail,
    StorageError,
}

pub struct RegisterAccountUseCaseImpl<
    A: AccountFactory,
    AR: AccountRepository,
    AS: AuthenticationService,
    C: ContactRepository,
> {
    account_service: Arc<A>,
    account_repository: Arc<AR>,
    authentication_service: Arc<AS>,
    contact_repository: Arc<C>,
}

impl<A: AccountFactory, AR: AccountRepository, AS: AuthenticationService, C: ContactRepository>
    RegisterAccountUseCaseImpl<A, AR, AS, C>
{
    pub fn new(
        account_service: Arc<A>,
        account_repository: Arc<AR>,
        authentication_service: Arc<AS>,
        contact_repository: Arc<C>,
    ) -> Self {
        Self {
            account_service,
            account_repository,
            authentication_service,
            contact_repository,
        }
    }

    fn generate_temporary_password() -> String {
        let password_gen = PasswordGenerator::new()
            .length(8)
            .numbers(true)
            .lowercase_letters(true)
            .uppercase_letters(false)
            .spaces(false)
            .symbols(false)
            .exclude_similar_characters(true)
            .strict(true);
        password_gen.generate_one().unwrap()
    }
}

impl<A: AccountFactory, AR: AccountRepository, AS: AuthenticationService, C: ContactRepository>
    RegisterAccountUseCase for RegisterAccountUseCaseImpl<A, AR, AS, C>
{
    fn register_account(&self, username: String, email: String) -> Result<(), CreateAccountError> {
        let account = match self.account_service.create_account(&username) {
            Ok(acc) => acc,
            Err(crate::domain::account::CreateAccountError::InvalidUsername) => {
                return Err(CreateAccountError::InvalidUsername);
            }
        };

        let password = Self::generate_temporary_password();
        let client_id = ClientId::new(&username);
        let client_secret = password;

        if let Err(e) = self
            .authentication_service
            .register_credentials(&client_id, &client_secret)
        {
            return Err(match e {
                AuthRegisterError::IdentifierTaken => CreateAccountError::UsernameTaken,
                AuthRegisterError::StorageError => CreateAccountError::StorageError,
            });
        }

        if let Err(e) = self
            .contact_repository
            .store_email_contact(&client_id, &email)
        {
            //TODO: log if rollback fails
            self.authentication_service.remove_credentials(&client_id);
            return Err(match e {
                StoreEmailError::InvalidEmail => CreateAccountError::InvalidEmail,
                StoreEmailError::StorageError => CreateAccountError::StorageError,
            });
        }

        if let Err(e) = self.account_repository.create_account(account) {
            //TODO: log if rollback fails
            self.authentication_service.remove_credentials(&client_id);
            self.contact_repository.remove_email_contact(&client_id);
            return Err(match e {
                CreateAccountRepoError::UsernameTaken => CreateAccountError::UsernameTaken,
                CreateAccountRepoError::StorageError => CreateAccountError::StorageError,
            });
        }
        Ok(())
    }
}
