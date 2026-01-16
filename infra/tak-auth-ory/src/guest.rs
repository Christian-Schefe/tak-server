use std::{collections::HashMap, sync::RwLock, time::Duration};

use tak_server_app::{
    domain::{
        AccountId,
        moderation::{AccountRole, ModerationFlags},
    },
    ports::authentication::{Account, AccountType},
};

struct SharedGuestRegistry {
    guest_account_ids: HashMap<String, AccountId>,
    guest_accounts: HashMap<AccountId, GuestAccount>,
    guest_usernames: HashMap<String, AccountId>,
    guest_number: u32,
}

#[derive(Debug, Clone)]
pub struct GuestAccount {
    pub account: Account,
    pub token: Option<String>,
    pub last_access: std::time::Instant,
}

pub struct GuestRegistry {
    inner: RwLock<SharedGuestRegistry>,
}

impl GuestRegistry {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(SharedGuestRegistry {
                guest_account_ids: HashMap::new(),
                guest_accounts: HashMap::new(),
                guest_usernames: HashMap::new(),
                guest_number: 0,
            }),
        }
    }

    pub fn get_or_create_guest(&self, token: Option<&str>) -> Account {
        let mut registry = self.inner.write().unwrap();

        let id = if let Some(token) = token {
            registry
                .guest_account_ids
                .entry(token.to_string())
                .or_insert_with(|| AccountId::new())
                .clone()
        } else {
            AccountId::new()
        };
        if let Some(guest) = registry.guest_accounts.get_mut(&id) {
            guest.last_access = std::time::Instant::now();
            return guest.account.clone();
        }

        let next_id = registry.guest_number;
        registry.guest_number += 1;
        let guest = GuestAccount {
            account: Account::new(
                id.clone(),
                AccountType::Guest,
                AccountRole::User,
                ModerationFlags::new(),
                format!("Guest{}", next_id),
                format!("Guest {}", next_id),
                None,
            ),
            token: token.map(|t| t.to_string()),
            last_access: std::time::Instant::now(),
        };
        registry.guest_accounts.insert(id.clone(), guest.clone());
        registry.guest_usernames.insert(
            guest.account.username.clone(),
            guest.account.account_id.clone(),
        );
        guest.account
    }

    pub fn get_by_username(&self, username: &str) -> Option<Account> {
        let registry = self.inner.read().unwrap();
        if let Some(account_id) = registry.guest_usernames.get(username) {
            if let Some(guest) = registry.guest_accounts.get(account_id) {
                return Some(guest.account.clone());
            }
        }
        None
    }

    pub fn get_by_id(&self, account_id: &AccountId) -> Option<Account> {
        let registry = self.inner.read().unwrap();
        if let Some(guest) = registry.guest_accounts.get(&account_id) {
            return Some(guest.account.clone());
        }
        None
    }

    pub fn clean_up_guest_accounts(&self) -> Vec<AccountId> {
        let mut registry = self.inner.write().unwrap();
        let removed_accounts: Vec<_> = registry
            .guest_accounts
            .extract_if(|_, val| val.last_access.elapsed() > Duration::from_secs(60 * 60 * 24))
            .collect();

        let mut removed_account_ids = Vec::new();
        for (account_id, guest) in removed_accounts {
            registry.guest_accounts.remove(&account_id);
            registry.guest_usernames.remove(&guest.account.username);
            if let Some(guest_token) = &guest.token {
                registry.guest_account_ids.remove(guest_token);
            }
            removed_account_ids.push(account_id);
        }

        removed_account_ids
    }

    pub fn update_guest<R>(
        &self,
        account_id: &AccountId,
        update: impl FnOnce(&mut Account) -> R,
    ) -> Option<R> {
        let mut registry = self.inner.write().unwrap();
        if let Some(guest) = registry.guest_accounts.get_mut(account_id) {
            Some(update(&mut guest.account))
        } else {
            None
        }
    }
}
