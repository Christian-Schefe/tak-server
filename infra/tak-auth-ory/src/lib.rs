use std::{sync::Arc, time::Duration};

use tak_server_api::ApiAuthPort;
use tak_server_app::{
    domain::{
        AccountId,
        moderation::{AccountRole, ModerationFlag},
    },
    ports::authentication::{Account, AuthenticationPort},
};
use tak_server_legacy_api::acl::LegacyApiAuthPort;

use crate::{
    bot::{BotRegistry, BotRepository},
    guest::GuestRegistry,
    ory::OryAuthenticationService,
};

pub mod bot;
mod guest;
pub mod jwt;
pub mod ory;

pub struct AuthenticationService {
    guest_registry: Arc<GuestRegistry>,
    bot_registry: Arc<BotRegistry>,
    ory_service: Arc<OryAuthenticationService>,
    account_cache: Arc<moka::sync::Cache<AccountId, Account>>,
    username_cache: Arc<moka::sync::Cache<String, Account>>,
}

impl AuthenticationService {
    pub fn new<R: BotRepository>(bot_repository: Arc<R>) -> Self {
        Self {
            guest_registry: Arc::new(GuestRegistry::new()),
            bot_registry: Arc::new(BotRegistry::new(bot_repository)),
            ory_service: Arc::new(OryAuthenticationService::new()),
            account_cache: Arc::new(
                moka::sync::Cache::builder()
                    .max_capacity(10_000)
                    .time_to_live(Duration::from_secs(60 * 10))
                    .build(),
            ),
            username_cache: Arc::new(
                moka::sync::Cache::builder()
                    .max_capacity(10_000)
                    .time_to_live(Duration::from_secs(60 * 10))
                    .build(),
            ),
        }
    }

    pub async fn find_by_username(&self, username: &str) -> Option<Account> {
        if let Some(cached_account) = self.username_cache.get(username) {
            return Some(cached_account);
        }
        let acc = if let Some(guest_account) = self.guest_registry.get_by_username(username) {
            Some(guest_account)
        } else if let Some(bot_account) = self.bot_registry.get_by_username(username) {
            Some(bot_account)
        } else {
            self.ory_service.find_by_username(username).await
        }?;
        self.username_cache
            .insert(username.to_string(), acc.clone());
        Some(acc)
    }

    async fn update_account<F>(
        &self,
        account_id: &AccountId,
        update_fn: impl FnOnce(&mut Account) + Copy,
        ory_update_fn: F,
    ) -> Result<(), ()>
    where
        F: AsyncFnOnce() -> Result<(), ()>,
    {
        if self
            .guest_registry
            .update_guest(&account_id, update_fn)
            .is_none()
        {
            if self
                .bot_registry
                .update_bot(&account_id, update_fn)
                .is_none()
            {
                ory_update_fn().await?;
            }
        }
        self.account_cache.invalidate(account_id);
        Ok(())
    }
}

#[async_trait::async_trait]
impl ApiAuthPort for AuthenticationService {
    async fn get_account_by_kratos_cookie(&self, token: &str) -> Option<Account> {
        self.ory_service.get_account_by_cookie(token).await
    }

    fn create_guest(&self) -> Account {
        self.guest_registry.get_or_create_guest(None)
    }

    fn generate_account_jwt(&self, id: &AccountId, duration: Duration) -> String {
        jwt::generate_jwt(id, duration)
    }

    fn validate_account_jwt(&self, token: &str) -> Option<AccountId> {
        if let Ok(claims) = jwt::Claims::from_token(token) {
            if let Ok(acc_id) = AccountId::try_from(claims.sub) {
                self.guest_registry.update_guest_last_access(&acc_id);
                Some(acc_id)
            } else {
                None
            }
        } else {
            None
        }
    }

    async fn get_account_by_username(&self, username: &str) -> Option<Account> {
        self.find_by_username(username).await
    }
}

#[async_trait::async_trait]
impl LegacyApiAuthPort for AuthenticationService {
    async fn get_or_create_guest_account(&self, token: &str) -> Account {
        self.guest_registry.get_or_create_guest(Some(token))
    }

    async fn find_by_username(&self, username: &str) -> Option<Account> {
        self.find_by_username(username).await
    }

    async fn create_account(
        &self,
        username: &str,
        email: &str,
        password_hash: &str,
    ) -> Result<Account, String> {
        self.ory_service
            .create_account(username, email, password_hash)
            .await
    }

    async fn login_username_password(
        &self,
        username: &str,
        password: &str,
    ) -> Result<Account, String> {
        self.ory_service
            .login_username_password(username, password)
            .await
    }

    async fn change_password(
        &self,
        username: &str,
        old_password: &str,
        new_password: &str,
    ) -> Result<(), String> {
        self.ory_service
            .change_password(username, old_password, new_password)
            .await
    }
}

#[async_trait::async_trait]
impl AuthenticationPort for AuthenticationService {
    async fn clean_up_guest_accounts(&self) -> Vec<AccountId> {
        self.guest_registry.clean_up_guest_accounts()
    }

    async fn get_account(&self, account_id: &AccountId) -> Option<Account> {
        if let Some(cached_account) = self.account_cache.get(account_id) {
            return Some(cached_account);
        }
        let account = if let Some(guest_account) = self.guest_registry.get_by_id(account_id) {
            guest_account
        } else if let Some(bot_account) = self.bot_registry.get_by_id(account_id) {
            bot_account
        } else {
            self.ory_service.get_account(account_id).await?
        };
        self.account_cache
            .insert(account_id.clone(), account.clone());
        Some(account)
    }

    async fn set_role(&self, account_id: &AccountId, role: AccountRole) -> Result<(), ()> {
        self.update_account(
            account_id,
            |account| account.role = role,
            || self.ory_service.set_role(account_id, role),
        )
        .await
    }

    async fn add_flag(&self, account_id: &AccountId, flag: ModerationFlag) -> Result<(), ()> {
        self.update_account(
            account_id,
            |account| account.add_flag(flag),
            || self.ory_service.add_flag(account_id, flag),
        )
        .await
    }

    async fn remove_flag(&self, account_id: &AccountId, flag: ModerationFlag) -> Result<(), ()> {
        self.update_account(
            account_id,
            |account| account.remove_flag(flag),
            || self.ory_service.remove_flag(account_id, flag),
        )
        .await
    }
}
