use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use dashmap::DashMap;
use ory_kratos_client::apis::{configuration::Configuration, identity_api::get_identity};
use tak_server_app::{
    domain::{
        AccountId,
        moderation::{AccountRole, ModerationFlag},
    },
    ports::authentication::{Account, AccountQuery, AccountType, AuthenticationPort},
};

pub struct OryAuthenticationService {
    guest_account_ids: Arc<DashMap<String, AccountId>>,
    guest_accounts: Arc<DashMap<AccountId, Account>>,
    guest_number: Arc<Mutex<u32>>,

    account_cache: Arc<moka::sync::Cache<AccountId, Account>>,

    ory_config: Arc<Configuration>,
}

#[derive(serde::Deserialize)]
pub enum OryAccountRole {
    User,
    Moderator,
    Admin,
}

#[derive(serde::Deserialize)]
pub enum OryModerationFlag {
    Banned,
    Silenced,
}

#[derive(serde::Deserialize)]
pub enum OryAccountType {
    Player,
    Bot,
}

#[derive(serde::Deserialize)]
pub struct OryTraits {
    pub email: Option<String>,
    pub username: String,
    pub display_name: String,
}

#[derive(serde::Deserialize)]
pub struct OryAdminMetadata {
    pub role: OryAccountRole,
    pub flags: Vec<OryModerationFlag>,
    pub account_type: OryAccountType,
}

impl OryAuthenticationService {
    pub fn new() -> Self {
        Self {
            guest_accounts: Arc::new(DashMap::new()),
            guest_account_ids: Arc::new(DashMap::new()),
            guest_number: Arc::new(Mutex::new(1)),
            account_cache: Arc::new(
                moka::sync::Cache::builder()
                    .max_capacity(1000)
                    .time_to_live(Duration::from_secs(60 * 60 * 12))
                    .build(),
            ),
            ory_config: Arc::new(Configuration {
                base_path: "localhost:4433".to_string(),
                client: reqwest::Client::new(),
                ..Default::default()
            }),
        }
    }

    fn take_guest_number(&self) -> u32 {
        let mut number_lock = self.guest_number.lock().unwrap();
        let number = *number_lock;
        *number_lock += 1;
        number
    }
}

#[async_trait::async_trait]
impl AuthenticationPort for OryAuthenticationService {
    async fn get_or_create_guest_account(&self, token: &str) -> Account {
        let id = self
            .guest_account_ids
            .entry(token.to_string())
            .or_insert_with(|| AccountId::new())
            .clone();
        let id_clone = id.clone();
        let account = self.guest_accounts.entry(id_clone).or_insert_with(|| {
            let guest_number = self.take_guest_number();
            Account::new(
                id,
                AccountType::Guest,
                AccountRole::User,
                format!("Guest{}", guest_number),
                None,
            )
        });
        account.clone()
    }

    async fn get_account(&self, account_id: &AccountId) -> Option<Account> {
        if let Some(guest_account) = self.guest_accounts.get(account_id) {
            return Some(guest_account.clone());
        }
        if let Some(cached_account) = self.account_cache.get(account_id) {
            return Some(cached_account);
        }

        let identity =
            match get_identity(self.ory_config.as_ref(), &account_id.to_string(), None).await {
                Ok(response) => response,
                Err(_) => return None,
            };

        let metadata: OryAdminMetadata = identity
            .metadata_admin
            .flatten()
            .map(|x| serde_json::from_value(x))
            .transpose()
            .unwrap_or(None)?;

        let traits: OryTraits = identity
            .traits
            .map(|x| serde_json::from_value(x))
            .transpose()
            .unwrap_or(None)?;

        let account_type = match metadata.account_type {
            OryAccountType::Player => AccountType::Player,
            OryAccountType::Bot => AccountType::Bot,
        };
        let role = match metadata.role {
            OryAccountRole::User => AccountRole::User,
            OryAccountRole::Moderator => AccountRole::Moderator,
            OryAccountRole::Admin => AccountRole::Admin,
        };

        let account = Account::new(
            AccountId(identity.id),
            account_type,
            role,
            traits.username,
            traits.email,
        );

        self.account_cache
            .insert(account_id.clone(), account.clone());
        Some(account)
    }

    async fn set_role(&self, account_id: &AccountId, role: AccountRole) -> Result<(), ()> {
        if let Some(mut guest_account) = self.guest_accounts.get_mut(&account_id) {
            guest_account.role = role;
            Ok(())
        } else {
            self.account_cache.invalidate(account_id);
            Ok(())
        }
    }

    async fn add_flag(&self, account_id: &AccountId, flag: ModerationFlag) -> Result<(), ()> {
        if let Some(mut guest_account) = self.guest_accounts.get_mut(&account_id) {
            guest_account.add_flag(flag);
            Ok(())
        } else {
            self.account_cache.invalidate(account_id);
            Ok(())
        }
    }

    async fn remove_flag(&self, account_id: &AccountId, flag: ModerationFlag) -> Result<(), ()> {
        if let Some(mut guest_account) = self.guest_accounts.get_mut(&account_id) {
            guest_account.remove_flag(flag);
            Ok(())
        } else {
            self.account_cache.invalidate(account_id);
            Ok(())
        }
    }

    async fn query_accounts(&self, query: AccountQuery) -> Vec<Account> {
        let mut results = Vec::new();
        for guest_account in self.guest_accounts.iter() {
            if let Some(flag) = query.flag {
                if !guest_account.is_flagged(flag) {
                    continue;
                }
            }
            if let Some(role) = query.role {
                if guest_account.role != role {
                    continue;
                }
            }
            if let Some(account_type) = query.account_type {
                if guest_account.account_type != account_type {
                    continue;
                }
            }
            results.push(guest_account.clone());
        }
        results
    }
}
