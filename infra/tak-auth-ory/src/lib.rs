use std::{sync::Arc, time::Duration};

use tak_server_api::ApiAuthPort;
use tak_server_app::{
    domain::{
        AccountId,
        moderation::{AccountRole, ModerationFlag},
    },
    ports::authentication::{Account, AuthenticationPort},
};

use crate::{guest::GuestRegistry, ory::OryAuthenticationService};

mod guest;
mod jwt;
mod ory;

pub struct AuthenticationService {
    guest_registry: Arc<GuestRegistry>,
    ory_service: Arc<OryAuthenticationService>,
    account_cache: Arc<moka::sync::Cache<AccountId, Account>>,
}

impl AuthenticationService {
    pub fn new() -> Self {
        Self {
            guest_registry: Arc::new(GuestRegistry::new()),
            ory_service: Arc::new(OryAuthenticationService::new()),
            account_cache: Arc::new(
                moka::sync::Cache::builder()
                    .max_capacity(10_000)
                    .time_to_live(Duration::from_secs(60 * 10))
                    .build(),
            ),
        }
    }

    pub async fn create_account(
        &self,
        username: &str,
        email: &str,
        password_hash: &str,
    ) -> Result<Account, String> {
        self.ory_service
            .create_account(username, email, password_hash)
            .await
    }

    pub async fn login_username_password(
        &self,
        username: &str,
        password: &str,
    ) -> Result<Account, String> {
        self.ory_service
            .login_username_password(username, password)
            .await
    }

    pub async fn change_password(
        &self,
        username: &str,
        old_password: &str,
        new_password: &str,
    ) -> Result<(), String> {
        self.ory_service
            .change_password(username, old_password, new_password)
            .await
    }

    pub async fn find_by_username(&self, username: &str) -> Option<Account> {
        if let Some(guest_account) = self.guest_registry.get_by_username(username) {
            return Some(guest_account);
        }
        self.ory_service.find_by_username(username).await
    }
}

#[async_trait::async_trait]
impl ApiAuthPort for AuthenticationService {
    async fn get_account_by_kratos_cookie(&self, token: &str) -> Option<Account> {
        self.ory_service.get_account_by_cookie(token).await
    }

    fn get_account_by_guest_jwt(&self, token: &str) -> Option<Account> {
        if let Ok(claims) = jwt::Claims::from_token(token)
            && let Some(account) = self.guest_registry.get_by_id(&AccountId(claims.sub))
        {
            Some(account)
        } else {
            None
        }
    }

    fn generate_or_refresh_guest_jwt(&self, token: Option<&str>) -> String {
        let account_id = if let Some(token) = token
            && let Some(account) = self.get_account_by_guest_jwt(token)
        {
            account.account_id
        } else {
            let account = self.guest_registry.get_or_create_guest(None);
            account.account_id
        };
        jwt::generate_jwt(&account_id)
    }
}

#[async_trait::async_trait]
impl AuthenticationPort for AuthenticationService {
    async fn get_or_create_guest_account(&self, token: &str) -> Account {
        self.guest_registry.get_or_create_guest(Some(token))
    }

    async fn clean_up_guest_accounts(&self) -> Vec<AccountId> {
        self.guest_registry.clean_up_guest_accounts()
    }

    async fn get_account(&self, account_id: &AccountId) -> Option<Account> {
        if let Some(cached_account) = self.account_cache.get(account_id) {
            return Some(cached_account);
        }
        let account = if let Some(guest_account) = self.guest_registry.get_by_id(account_id) {
            guest_account
        } else {
            self.ory_service.get_account(account_id).await?
        };
        self.account_cache
            .insert(account_id.clone(), account.clone());
        Some(account)
    }

    async fn set_role(&self, account_id: &AccountId, role: AccountRole) -> Result<(), ()> {
        if self
            .guest_registry
            .update_guest(&account_id, |account| {
                account.role = role;
            })
            .is_none()
        {
            self.ory_service
                .set_role(account_id, role)
                .await
                .map_err(|_| ())?;
        }
        self.account_cache.invalidate(account_id);
        Ok(())
    }

    async fn add_flag(&self, account_id: &AccountId, flag: ModerationFlag) -> Result<(), ()> {
        if self
            .guest_registry
            .update_guest(&account_id, |account| {
                account.add_flag(flag);
            })
            .is_none()
        {
            self.ory_service
                .add_flag(account_id, flag)
                .await
                .map_err(|_| ())?;
        }
        self.account_cache.invalidate(account_id);
        Ok(())
    }

    async fn remove_flag(&self, account_id: &AccountId, flag: ModerationFlag) -> Result<(), ()> {
        if self
            .guest_registry
            .update_guest(&account_id, |account| {
                account.remove_flag(flag);
            })
            .is_none()
        {
            self.ory_service
                .remove_flag(account_id, flag)
                .await
                .map_err(|_| ())?;
        }
        self.account_cache.invalidate(account_id);
        Ok(())
    }
}
