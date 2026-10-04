use std::{sync::Arc, time::Duration};

use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use tak_server_api::ApiAuthPort;
use tak_server_app::{
    domain::{
        AccountId, RepoError, RepoRetrieveError,
        moderation::{AccountRole, ModerationFlag, ModerationFlags},
    },
    ports::authentication::{Account, AccountType, AuthenticationPort},
};

mod jwt;

#[async_trait::async_trait]
pub trait AccountRepository {
    async fn create_account(&self, account: FullAccount) -> Result<(), RepoError>;
    async fn get_account(&self, account_id: &AccountId) -> Result<FullAccount, RepoRetrieveError>;
    async fn update_account(&self, account: FullAccount) -> Result<(), RepoError>;
    async fn get_account_by_username(
        &self,
        username: &str,
    ) -> Result<FullAccount, RepoRetrieveError>;
    async fn get_account_by_email(&self, email: &str) -> Result<FullAccount, RepoRetrieveError>;
}

pub struct FullAccount {
    pub account_id: AccountId,
    pub account: Account,
    pub password_hash: Option<String>,
}

pub struct AuthService<AR: AccountRepository> {
    account_repo: Arc<AR>,
    argon2: Argon2<'static>,
}

impl<AR: AccountRepository> AuthService<AR> {
    pub fn new(account_repo: Arc<AR>) -> Self {
        Self {
            account_repo,
            argon2: Argon2::default(),
        }
    }
}

#[async_trait::async_trait]
impl<AR> ApiAuthPort for AuthService<AR>
where
    AR: AccountRepository + Send + Sync + 'static,
{
    async fn register_account(
        &self,
        email: &str,
        username: &str,
        password: &str,
    ) -> Option<(AccountId, Account)> {
        let account = Account::new(
            AccountType::Player,
            AccountRole::User,
            ModerationFlags::empty(),
            username.to_string(),
            Some(email.to_string()),
        );
        let password_hash = match self.argon2.hash_password(password.as_bytes()) {
            Ok(hash) => hash.to_string(),
            Err(e) => {
                tracing::error!("Failed to hash password: {}", e);
                return None;
            }
        };
        let account_id = AccountId::new();
        let full_account = FullAccount {
            account_id: account_id.clone(),
            account,
            password_hash: Some(password_hash),
        };

        match self.account_repo.create_account(full_account).await {
            Ok(()) => match self.account_repo.get_account(&account_id).await {
                Ok(full_account) => Some((account_id, full_account.account)),
                Err(RepoRetrieveError::NotFound) => None,
                Err(RepoRetrieveError::StorageError(e)) => {
                    tracing::error!("Failed to retrieve account {}: {}", account_id, e);
                    None
                }
            },
            Err(e) => {
                tracing::error!("Failed to create account: {}", e);
                None
            }
        }
    }

    async fn login_account(
        &self,
        identifier: &str,
        password: &str,
    ) -> Option<(AccountId, Account)> {
        let is_email = identifier.contains('@');
        let full_account = match if is_email {
            self.account_repo.get_account_by_email(identifier).await
        } else {
            self.account_repo.get_account_by_username(identifier).await
        } {
            Ok(full_account) => full_account,
            Err(RepoRetrieveError::NotFound) => return None,
            Err(RepoRetrieveError::StorageError(e)) => {
                tracing::error!("Failed to retrieve account {}: {}", identifier, e);
                return None;
            }
        };
        let Some(password_hash) = &full_account.password_hash else {
            return None;
        };
        match self
            .argon2
            .verify_password(password.as_bytes(), password_hash.as_str())
        {
            Ok(()) => Some((full_account.account_id, full_account.account)),
            Err(e) => {
                tracing::error!("Failed to verify password: {}", e);
                None
            }
        }
    }

    async fn create_guest_account(&self) -> Option<(AccountId, Account)> {
        let account_id = AccountId::new();
        let account = Account::new(
            AccountType::Guest,
            AccountRole::User,
            ModerationFlags::empty(),
            format!("guest_{}", account_id.to_string()),
            None,
        );
        let full_account = FullAccount {
            account_id: account_id.clone(),
            account,
            password_hash: None,
        };
        match self.account_repo.create_account(full_account).await {
            Ok(()) => match self.account_repo.get_account(&account_id).await {
                Ok(full_account) => Some((account_id, full_account.account)),
                Err(RepoRetrieveError::NotFound) => None,
                Err(RepoRetrieveError::StorageError(e)) => {
                    tracing::error!("Failed to retrieve account {}: {}", account_id, e);
                    None
                }
            },
            Err(e) => {
                tracing::error!("Failed to create guest account: {}", e);
                None
            }
        }
    }

    fn generate_account_jwt(&self, id: &AccountId, duration: Duration) -> String {
        jwt::generate_jwt(id, duration)
    }

    async fn validate_account_jwt(&self, token: &str) -> Option<(AccountId, Account)> {
        let account_id = jwt::validate_jwt(token)?;
        match self.account_repo.get_account(&account_id).await {
            Ok(full_account) => Some((account_id, full_account.account)),
            Err(RepoRetrieveError::NotFound) => None,
            Err(RepoRetrieveError::StorageError(e)) => {
                tracing::error!("Failed to retrieve account {}: {}", account_id, e);
                None
            }
        }
    }
}

#[async_trait::async_trait]
impl<AR> AuthenticationPort for AuthService<AR>
where
    AR: AccountRepository + Send + Sync + 'static,
{
    async fn get_account(&self, account_id: &AccountId) -> Option<Account> {
        match self.account_repo.get_account(account_id).await {
            Ok(full_account) => Some(full_account.account),
            Err(RepoRetrieveError::NotFound) => None,
            Err(RepoRetrieveError::StorageError(e)) => {
                tracing::error!("Failed to retrieve account {}: {}", account_id, e);
                None
            }
        }
    }
    async fn set_role(&self, account_id: &AccountId, role: AccountRole) -> Result<(), ()> {
        let mut full_account = match self.account_repo.get_account(account_id).await {
            Ok(full_account) => full_account,
            Err(RepoRetrieveError::NotFound) => return Err(()),
            Err(RepoRetrieveError::StorageError(e)) => {
                tracing::error!("Failed to retrieve account {}: {}", account_id, e);
                return Err(());
            }
        };
        full_account.account.role = role;
        match self.account_repo.update_account(full_account).await {
            Ok(()) => Ok(()),
            Err(e) => {
                tracing::error!("Failed to update account {}: {}", account_id, e);
                Err(())
            }
        }
    }
    async fn add_flag(&self, account_id: &AccountId, flag: ModerationFlag) -> Result<(), ()> {
        let mut full_account = match self.account_repo.get_account(account_id).await {
            Ok(full_account) => full_account,
            Err(RepoRetrieveError::NotFound) => return Err(()),
            Err(RepoRetrieveError::StorageError(e)) => {
                tracing::error!("Failed to retrieve account {}: {}", account_id, e);
                return Err(());
            }
        };
        full_account.account.add_flag(flag);
        match self.account_repo.update_account(full_account).await {
            Ok(()) => Ok(()),
            Err(e) => {
                tracing::error!("Failed to update account {}: {}", account_id, e);
                Err(())
            }
        }
    }
    async fn remove_flag(&self, account_id: &AccountId, flag: ModerationFlag) -> Result<(), ()> {
        let mut full_account = match self.account_repo.get_account(account_id).await {
            Ok(full_account) => full_account,
            Err(RepoRetrieveError::NotFound) => return Err(()),
            Err(RepoRetrieveError::StorageError(e)) => {
                tracing::error!("Failed to retrieve account {}: {}", account_id, e);
                return Err(());
            }
        };
        full_account.account.remove_flag(flag);
        match self.account_repo.update_account(full_account).await {
            Ok(()) => Ok(()),
            Err(e) => {
                tracing::error!("Failed to update account {}: {}", account_id, e);
                Err(())
            }
        }
    }
}
