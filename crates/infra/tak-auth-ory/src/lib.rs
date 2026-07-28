use std::{sync::Arc, time::Duration};

use tak_server_api::ApiAuthPort;
use tak_server_app::{
    domain::{
        AccountId,
        moderation::{AccountRole, ModerationFlag},
    },
    ports::authentication::{Account, AuthenticationPort},
};

use crate::{
    bot::{BotRegistry, BotRepository},
    guest::{GuestRepository, GuestService},
    ory::OryAuthenticationService,
};

pub mod bot;
pub mod guest;
pub mod jwt;
mod ory;

pub struct AuthenticationService {
    guest_service: Arc<GuestService>,
    bot_registry: Arc<BotRegistry>,
    ory_service: Arc<OryAuthenticationService>,
    account_cache: Arc<moka::sync::Cache<AccountId, Account>>,
    username_cache: Arc<moka::sync::Cache<String, Account>>,
}

impl AuthenticationService {
    pub fn new<R: BotRepository, G: GuestRepository + Send + Sync + 'static>(
        bot_repository: Arc<R>,
        guest_repository: Arc<G>,
    ) -> Self {
        Self {
            guest_service: Arc::new(GuestService::new(guest_repository)),
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
        let acc = if let Some(guest_account) = self.guest_service.get_by_username(username).await {
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
            .guest_service
            .update_guest(&account_id, update_fn)
            .await
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

    async fn create_guest(&self) -> Option<Account> {
        self.guest_service.create_guest().await
    }

    fn generate_account_jwt(&self, id: &AccountId, duration: Duration) -> String {
        jwt::generate_jwt(id, duration)
    }

    async fn validate_account_jwt(&self, token: &str) -> Option<Account> {
        if let Ok(claims) = jwt::Claims::from_token(token) {
            self.get_account(&AccountId::try_from(claims.sub).ok()?).await
        } else {
            None
        }
    }

    async fn get_account_by_username(&self, username: &str) -> Option<Account> {
        self.find_by_username(username).await
    }
}

#[async_trait::async_trait]
impl AuthenticationPort for AuthenticationService {
    async fn get_account(&self, account_id: &AccountId) -> Option<Account> {
        if let Some(cached_account) = self.account_cache.get(account_id) {
            return Some(cached_account);
        }
        let account =
            if let Some(guest_account) = self.guest_service.get_by_account_id(account_id).await {
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
