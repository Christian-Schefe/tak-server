use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use rand::{TryRngCore, rngs::OsRng};
use sha2::Digest;
use tak_domain::UserId;
use tak_server_app::{
    errors::{AppResult, EmailTaken, InternalError, InvalidCredentials, OneOf},
    ports::authentication::{AccountInfo, AuthenticationPort, RefreshToken},
};

mod guest;

#[async_trait::async_trait]
pub trait AccountRepository {
    async fn create_account(&self, account: Account) -> AppResult<(), (EmailTaken, InternalError)>;
    async fn get_account(&self, user_id: UserId) -> AppResult<Option<Account>, (InternalError,)>;
    async fn update_account(&self, account: Account) -> AppResult<(), (InternalError,)>;
    async fn get_account_by_username(
        &self,
        username: &str,
    ) -> AppResult<Option<Account>, (InternalError,)>;
    async fn get_account_by_email(
        &self,
        email: &str,
    ) -> AppResult<Option<Account>, (InternalError,)>;
}

pub struct Account {
    pub user_id: UserId,
    pub username: String,
    pub email: Option<String>,
    pub password_hash: Option<String>,
    pub unique_name: Option<String>,
}

pub struct AuthService<AR: AccountRepository> {
    account_repo: Arc<AR>,
    argon2: Argon2<'static>,
    reset_token_registry: Arc<Mutex<HashMap<UserId, (Vec<u8>, DateTime<Utc>)>>>,
    refresh_token_registry: Arc<Mutex<HashMap<UserId, (Vec<u8>, DateTime<Utc>)>>>,
}

fn evict_expired_tokens<T>(registry: &mut HashMap<UserId, (T, DateTime<Utc>)>) {
    let now = Utc::now();
    registry.retain(|_, (_, expiry)| *expiry > now);
}

impl<AR: AccountRepository> AuthService<AR> {
    pub fn new(account_repo: Arc<AR>) -> Self {
        Self {
            account_repo,
            argon2: Argon2::default(),
            reset_token_registry: Arc::new(Mutex::new(HashMap::new())),
            refresh_token_registry: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

async fn hash_password(
    argon2: &Argon2<'static>,
    password: &str,
) -> AppResult<String, (InternalError,)> {
    tokio::task::spawn_blocking({
        let argon2 = argon2.clone();
        let password = password.to_string();
        move || {
            argon2
                .hash_password(password.as_bytes())
                .map_err(|e| {
                    tracing::error!("Failed to hash password: {:?}", e);
                    OneOf::new(InternalError)
                })
                .map(|hash| hash.to_string())
        }
    })
    .await
    .map_err(|e| {
        tracing::error!("Failed to join hash_password task: {:?}", e);
        OneOf::new(InternalError)
    })?
}
async fn verify_password(
    argon2: &Argon2<'static>,
    password: &str,
    password_hash: &str,
) -> AppResult<bool, (InternalError,)> {
    tokio::task::spawn_blocking({
        let argon2 = argon2.clone();
        let password = password.to_string();
        let password_hash = password_hash.to_string();
        move || match argon2.verify_password(password.as_bytes(), password_hash.as_str()) {
            Ok(()) => Ok(true),
            Err(argon2::password_hash::Error::PasswordInvalid) => Ok(false),
            Err(e) => {
                tracing::error!("Failed to verify password: {:?}", e);
                Err(OneOf::new(InternalError))
            }
        }
    })
    .await
    .map_err(|e| {
        tracing::error!("Failed to join verify_password task: {:?}", e);
        OneOf::new(InternalError)
    })?
}

#[async_trait::async_trait]
impl<AR> AuthenticationPort for AuthService<AR>
where
    AR: AccountRepository + Send + Sync + 'static,
{
    async fn register_account(
        &self,
        email: &str,
        username: &str,
        password: &str,
    ) -> AppResult<UserId, (EmailTaken, InternalError)> {
        let password_hash = hash_password(&self.argon2, password)
            .await
            .map_err(OneOf::broaden)?;
        let user_id = UserId::new();
        let account = Account {
            user_id: user_id.clone(),
            username: username.to_string(),
            email: Some(email.to_string()),
            password_hash: Some(password_hash),
            unique_name: Some(username.to_string()),
        };

        self.account_repo
            .create_account(account)
            .await
            .map_err(OneOf::broaden)?;
        Ok(user_id)
    }

    async fn login_account(
        &self,
        identifier: &str,
        password: &str,
    ) -> AppResult<UserId, (InvalidCredentials, InternalError)> {
        let is_email = identifier.contains('@');
        let account = match if is_email {
            self.account_repo.get_account_by_email(identifier).await
        } else {
            self.account_repo.get_account_by_username(identifier).await
        }
        .map_err(OneOf::broaden)?
        {
            Some(account) => account,
            None => return Err(OneOf::new(InvalidCredentials)),
        };
        let Some(password_hash) = &account.password_hash else {
            return Err(OneOf::new(InvalidCredentials));
        };
        if !verify_password(&self.argon2, password, password_hash)
            .await
            .map_err(OneOf::broaden)?
        {
            return Err(OneOf::new(InvalidCredentials));
        }
        Ok(account.user_id)
    }

    async fn find_account(&self, identifier: &str) -> AppResult<Option<UserId>, (InternalError,)> {
        let is_email = identifier.contains('@');
        let account = match if is_email {
            self.account_repo.get_account_by_email(identifier).await
        } else {
            self.account_repo.get_account_by_username(identifier).await
        }
        .map_err(OneOf::broaden)?
        {
            Some(account) => account,
            None => return Ok(None),
        };
        Ok(Some(account.user_id))
    }

    async fn create_guest_account(&self) -> AppResult<UserId, (InternalError,)> {
        let user_id = UserId::new();
        let name = guest::generate_guest_name();
        let account = Account {
            user_id: user_id.clone(),
            username: name,
            email: None,
            password_hash: None,
            unique_name: None,
        };
        self.account_repo
            .create_account(account)
            .await
            .map_err(|_| OneOf::new(InternalError))?;
        Ok(user_id)
    }
    fn generate_reset_token(
        &self,
        user_id: UserId,
        duration: Duration,
    ) -> AppResult<String, (InternalError,)> {
        let mut bytes = [0u8; 32];
        OsRng.try_fill_bytes(&mut bytes).map_err(|e| {
            tracing::error!("Failed to generate reset token: {}", e);
            OneOf::new(InternalError)
        })?;
        let token = URL_SAFE_NO_PAD.encode(bytes);
        let mut registry = self.reset_token_registry.lock().unwrap();
        evict_expired_tokens(&mut registry);
        let expiry = Utc::now()
            + chrono::Duration::from_std(duration).map_err(|e| {
                tracing::error!("Failed to calculate expiry time: {}", e);
                OneOf::new(InternalError)
            })?;
        let token_hash = sha2::Sha256::digest(token.as_bytes()).to_vec();
        registry.insert(user_id, (token_hash, expiry));
        Ok(token)
    }
    async fn reset_account_password(
        &self,
        user_id: UserId,
        token: &str,
        new_password: &str,
    ) -> AppResult<Option<()>, (InvalidCredentials, InternalError)> {
        {
            let mut registry = self.reset_token_registry.lock().unwrap();
            let Some((token_hash, expiry)) = registry.get(&user_id) else {
                return Ok(None);
            };
            if Utc::now() > *expiry {
                registry.remove(&user_id);
                return Ok(None);
            }
            let provided_token_hash = sha2::Sha256::digest(token.as_bytes()).to_vec();
            if provided_token_hash != *token_hash {
                return Ok(None);
            }
        }
        let new_password_hash = hash_password(&self.argon2, new_password)
            .await
            .map_err(OneOf::broaden)?;
        let mut account = match self
            .account_repo
            .get_account(user_id)
            .await
            .map_err(OneOf::broaden)?
        {
            Some(account) => account,
            None => return Ok(None),
        };
        account.password_hash = Some(new_password_hash);
        self.account_repo
            .update_account(account)
            .await
            .map_err(OneOf::broaden)?;
        {
            let mut registry = self.reset_token_registry.lock().unwrap();
            registry.remove(&user_id);
        }
        Ok(Some(()))
    }

    async fn get_account_info(
        &self,
        user_id: UserId,
    ) -> AppResult<Option<AccountInfo>, (InternalError,)> {
        Ok(self
            .account_repo
            .get_account(user_id)
            .await
            .map_err(OneOf::broaden)?
            .map(|account| AccountInfo {
                username: account.username,
                email: account.email,
            }))
    }

    fn generate_refresh_token(
        &self,
        user_id: UserId,
        duration: Duration,
    ) -> AppResult<RefreshToken, (InternalError,)> {
        let mut registry = self.refresh_token_registry.lock().unwrap();
        generate_token(&mut registry, user_id, duration)
    }
    fn rotate_refresh_token(
        &self,
        user_id: UserId,
        refresh_token: &str,
        duration: Duration,
    ) -> AppResult<RefreshToken, (InvalidCredentials, InternalError)> {
        let mut registry = self.refresh_token_registry.lock().unwrap();
        let Some((token_hash, expiry)) = registry.get(&user_id) else {
            return Err(OneOf::new(InvalidCredentials));
        };
        if Utc::now() > *expiry {
            registry.remove(&user_id);
            return Err(OneOf::new(InvalidCredentials));
        }
        let provided_token_hash = sha2::Sha256::digest(refresh_token.as_bytes()).to_vec();
        if provided_token_hash != *token_hash {
            return Err(OneOf::new(InvalidCredentials));
        }
        generate_token(&mut registry, user_id, duration).map_err(OneOf::broaden)
    }
    fn invalidate_refresh_token(&self, user_id: UserId) -> AppResult<(), (InternalError,)> {
        let mut registry = self.refresh_token_registry.lock().unwrap();
        registry.remove(&user_id);
        Ok(())
    }
}

fn generate_token(
    registry: &mut HashMap<UserId, (Vec<u8>, DateTime<Utc>)>,
    user_id: UserId,
    duration: Duration,
) -> AppResult<RefreshToken, (InternalError,)> {
    let mut bytes = [0u8; 32];
    OsRng.try_fill_bytes(&mut bytes).map_err(|e| {
        tracing::error!("Failed to generate refresh token: {}", e);
        OneOf::new(InternalError)
    })?;
    let token = URL_SAFE_NO_PAD.encode(bytes);
    evict_expired_tokens(registry);
    let expiry = Utc::now()
        + chrono::Duration::from_std(duration).map_err(|e| {
            tracing::error!("Failed to calculate expiry time: {}", e);
            OneOf::new(InternalError)
        })?;
    let token_hash = sha2::Sha256::digest(token.as_bytes()).to_vec();
    registry.insert(user_id, (token_hash, expiry));
    Ok(RefreshToken {
        token,
        valid_until: expiry,
    })
}
