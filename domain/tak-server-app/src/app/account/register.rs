use std::{sync::Arc, time::Duration};

use passwords::PasswordGenerator;

use crate::{
    domain::{
        account::{AccountFactory, AccountRepository, CreateAccountRepoError},
        player::{Player, PlayerRepository},
    },
    ports::{
        authentication::{AuthRegisterError, AuthenticationService, SessionToken, SubjectId},
        contact::{ContactRepository, StoreEmailError},
    },
};

pub trait RegisterAccountUseCase {
    fn register_account(&self, username: String, email: String) -> Result<(), CreateAccountError>;
    fn create_guest(&self) -> SessionToken;
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
    PR: PlayerRepository,
> {
    account_factory: Arc<A>,
    account_repository: Arc<AR>,
    authentication_service: Arc<AS>,
    contact_repository: Arc<C>,
    player_repository: Arc<PR>,
}

impl<
    A: AccountFactory,
    AR: AccountRepository,
    AS: AuthenticationService,
    C: ContactRepository,
    PR: PlayerRepository,
> RegisterAccountUseCaseImpl<A, AR, AS, C, PR>
{
    pub fn new(
        account_service: Arc<A>,
        account_repository: Arc<AR>,
        authentication_service: Arc<AS>,
        contact_repository: Arc<C>,
        player_repository: Arc<PR>,
    ) -> Self {
        Self {
            account_factory: account_service,
            account_repository,
            authentication_service,
            contact_repository,
            player_repository,
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

impl<
    A: AccountFactory,
    AR: AccountRepository,
    AS: AuthenticationService,
    C: ContactRepository,
    PR: PlayerRepository,
> RegisterAccountUseCase for RegisterAccountUseCaseImpl<A, AR, AS, C, PR>
{
    fn register_account(&self, username: String, email: String) -> Result<(), CreateAccountError> {
        let account = match self.account_factory.create_account(&username) {
            Ok(acc) => acc,
            Err(crate::domain::account::CreateAccountError::InvalidUsername) => {
                return Err(CreateAccountError::InvalidUsername);
            }
        };

        let player = Player::new();
        let player_id = player.player_id;
        self.player_repository.create_player(player);
        self.player_repository
            .link_account(player_id, account.account_id);

        let password = Self::generate_temporary_password();
        let subject_id = SubjectId::Account(account.account_id);
        if let Err(e) = self.authentication_service.register_username_password(
            &subject_id,
            &username,
            &password,
        ) {
            return Err(match e {
                AuthRegisterError::IdentifierTaken => CreateAccountError::UsernameTaken,
                AuthRegisterError::StorageError => CreateAccountError::StorageError,
            });
        }

        if let Err(e) = self
            .contact_repository
            .store_email_contact(&subject_id, &email)
        {
            //TODO: log if rollback fails
            self.player_repository.delete_player(player_id);
            self.authentication_service.revoke_credentials(&subject_id);
            return Err(match e {
                StoreEmailError::InvalidEmail => CreateAccountError::InvalidEmail,
                StoreEmailError::StorageError => CreateAccountError::StorageError,
            });
        }

        if let Err(e) = self.account_repository.create_account(account) {
            //TODO: log if rollback fails
            self.player_repository.delete_player(player_id);
            self.authentication_service.revoke_credentials(&subject_id);
            self.contact_repository.remove_email_contact(&subject_id);
            return Err(match e {
                CreateAccountRepoError::UsernameTaken => CreateAccountError::UsernameTaken,
                CreateAccountRepoError::StorageError => CreateAccountError::StorageError,
            });
        }
        Ok(())
    }

    fn create_guest(&self) -> SessionToken {
        let account = self.account_factory.create_guest_account();
        let subject_id = SubjectId::Account(account.account_id);

        let session_token = self
            .authentication_service
            .issue_session_token(&subject_id, Duration::from_secs(60 * 60 * 5));

        let player = Player::new();
        let player_id = player.player_id;
        self.player_repository.create_player(player);
        self.player_repository
            .link_account(player_id, account.account_id);
        session_token
    }
}
