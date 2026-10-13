use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use rand::{TryRngCore, rngs::OsRng};
use tak_server_api::ApiAuthPort;
use tak_server_app::{
    domain::{RepoError, RepoRetrieveError, UserId},
    ports::authentication::{AccountInfo, AccountPort},
};

mod guest;
mod jwt;

#[async_trait::async_trait]
pub trait AccountRepository {
    async fn create_account(&self, account: Account) -> Result<(), RepoError>;
    async fn get_account(&self, user_id: UserId) -> Result<Account, RepoRetrieveError>;
    async fn update_account(&self, account: Account) -> Result<(), RepoError>;
    async fn get_account_by_username(&self, username: &str) -> Result<Account, RepoRetrieveError>;
    async fn get_account_by_email(&self, email: &str) -> Result<Account, RepoRetrieveError>;
}

pub struct Account {
    pub user_id: UserId,
    pub username: String,
    pub email: Option<String>,
    pub password_hash: Option<String>,
    pub unique_name: Option<String>,
}

//TODO: Evict expired reset tokens from the registry periodically to prevent memory bloat.
pub struct AuthService<AR: AccountRepository> {
    account_repo: Arc<AR>,
    argon2: Argon2<'static>,
    reset_token_registry: Arc<Mutex<HashMap<UserId, (String, DateTime<Utc>)>>>,
}

impl<AR: AccountRepository> AuthService<AR> {
    pub fn new(account_repo: Arc<AR>) -> Self {
        Self {
            account_repo,
            argon2: Argon2::default(),
            reset_token_registry: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl<AR> ApiAuthPort for AuthService<AR>
where
    AR: AccountRepository + Send + Sync + 'static,
{
    fn generate_account_jwt(&self, id: UserId, duration: Duration) -> String {
        jwt::generate_jwt(id, duration)
    }

    fn validate_account_jwt(&self, token: &str) -> Option<UserId> {
        jwt::validate_jwt(token)
    }
}

#[async_trait::async_trait]
impl<AR> AccountPort for AuthService<AR>
where
    AR: AccountRepository + Send + Sync + 'static,
{
    async fn register_account(
        &self,
        email: &str,
        username: &str,
        password: &str,
    ) -> Result<UserId, String> {
        let password_hash = match self.argon2.hash_password(password.as_bytes()) {
            Ok(hash) => hash.to_string(),
            Err(e) => {
                tracing::error!("Failed to hash password: {}", e);
                return Err("Failed to hash password".into());
            }
        };
        let user_id = UserId::new();
        let account = Account {
            user_id: user_id.clone(),
            username: username.to_string(),
            email: Some(email.to_string()),
            password_hash: Some(password_hash),
            unique_name: Some(username.to_string()),
        };

        match self.account_repo.create_account(account).await {
            Ok(()) => Ok(user_id),
            Err(e) => {
                tracing::error!("Failed to create account: {}", e);
                Err("Failed to create account".into())
            }
        }
    }

    async fn login_account(
        &self,
        identifier: &str,
        password: &str,
    ) -> Result<Option<UserId>, String> {
        let is_email = identifier.contains('@');
        let account = match if is_email {
            self.account_repo.get_account_by_email(identifier).await
        } else {
            self.account_repo.get_account_by_username(identifier).await
        } {
            Ok(account) => account,
            Err(RepoRetrieveError::NotFound) => return Ok(None),
            Err(RepoRetrieveError::StorageError(e)) => {
                tracing::error!("Failed to retrieve account {}: {}", identifier, e);
                return Err("Failed to retrieve account".into());
            }
        };
        let Some(password_hash) = &account.password_hash else {
            return Ok(None);
        };
        match self
            .argon2
            .verify_password(password.as_bytes(), password_hash.as_str())
        {
            Ok(()) => Ok(Some(account.user_id)),
            Err(argon2::password_hash::Error::PasswordInvalid) => Ok(None),
            Err(e) => {
                tracing::error!("Failed to verify password: {}", e);
                Err("Failed to verify password".into())
            }
        }
    }

    async fn find_account(&self, identifier: &str) -> Result<Option<UserId>, String> {
        let is_email = identifier.contains('@');
        match if is_email {
            self.account_repo.get_account_by_email(identifier).await
        } else {
            self.account_repo.get_account_by_username(identifier).await
        } {
            Ok(account) => Ok(Some(account.user_id)),
            Err(RepoRetrieveError::NotFound) => Ok(None),
            Err(RepoRetrieveError::StorageError(e)) => {
                tracing::error!("Failed to retrieve account {}: {}", identifier, e);
                Err("Failed to retrieve account".into())
            }
        }
    }

    async fn create_guest_account(&self) -> Result<UserId, String> {
        let user_id = UserId::new();
        let name = guest::generate_guest_name();
        let account = Account {
            user_id: user_id.clone(),
            username: name,
            email: None,
            password_hash: None,
            unique_name: None,
        };
        match self.account_repo.create_account(account).await {
            Ok(()) => Ok(user_id),
            Err(e) => {
                tracing::error!("Failed to create guest account: {}", e);
                Err("Failed to create guest account".into())
            }
        }
    }
    fn generate_reset_token(&self, user_id: UserId, duration: Duration) -> Result<String, String> {
        let mut bytes = [0u8; 32];
        OsRng
            .try_fill_bytes(&mut bytes)
            .map_err(|_| "Failed to generate reset token".to_string())?;
        let token = URL_SAFE_NO_PAD.encode(bytes);
        let mut registry = self.reset_token_registry.lock().unwrap();
        let expiry = Utc::now()
            + chrono::Duration::from_std(duration)
                .map_err(|_| "Failed to calculate expiry time".to_string())?;
        let token_hash = self
            .argon2
            .hash_password(token.as_bytes())
            .map_err(|_| "Failed to hash reset token".to_string())?
            .to_string();
        registry.insert(user_id, (token_hash, expiry));
        Ok(token)
    }
    async fn reset_account_password(
        &self,
        user_id: UserId,
        token: &str,
        new_password: &str,
    ) -> Result<Option<()>, String> {
        {
            let mut registry = self.reset_token_registry.lock().unwrap();
            let Some((token_hash, expiry)) = registry.get(&user_id) else {
                return Ok(None);
            };
            if Utc::now() > *expiry {
                registry.remove(&user_id);
                return Ok(None);
            }
            match self
                .argon2
                .verify_password(token.as_bytes(), token_hash.as_str())
            {
                Ok(()) => {}
                Err(argon2::password_hash::Error::PasswordInvalid) => return Ok(None),
                Err(e) => {
                    tracing::error!("Failed to verify reset token: {}", e);
                    return Err("Failed to verify reset token".to_string());
                }
            }
            registry.remove(&user_id);
        }
        let new_password_hash = self
            .argon2
            .hash_password(new_password.as_bytes())
            .map_err(|_| "Failed to hash new password".to_string())?
            .to_string();
        let mut account = self
            .account_repo
            .get_account(user_id)
            .await
            .map_err(|_| "Failed to retrieve account".to_string())?;
        account.password_hash = Some(new_password_hash);
        self.account_repo
            .update_account(account)
            .await
            .map_err(|_| "Failed to update account".to_string())?;
        Ok(Some(()))
    }

    async fn get_account_info(&self, user_id: UserId) -> Result<Option<AccountInfo>, ()> {
        match self.account_repo.get_account(user_id).await {
            Ok(account) => Ok(Some(AccountInfo {
                username: account.username,
                email: account.email,
            })),
            Err(RepoRetrieveError::NotFound) => Ok(None),
            Err(RepoRetrieveError::StorageError(e)) => {
                tracing::error!("Failed to retrieve account {}: {}", user_id, e);
                Err(())
            }
        }
    }
}
