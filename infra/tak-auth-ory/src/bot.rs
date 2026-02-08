use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use tak_server_app::{
    domain::{
        AccountId,
        moderation::{AccountRole, ModerationFlags},
    },
    ports::authentication::{Account, AccountType},
};

pub trait BotRepository {
    fn get_bots(&self) -> impl Iterator<Item = &BotEntry>;
}

pub struct BotEntry {
    pub account_id: AccountId,
    pub username: String,
    pub display_name: String,
}

struct SharedBotRegistry {
    bot_accounts: HashMap<AccountId, Account>,
    bot_usernames: HashMap<String, AccountId>,
}

pub struct BotRegistry {
    inner: RwLock<SharedBotRegistry>,
}

impl BotRegistry {
    pub fn new<R: BotRepository>(bot_repository: Arc<R>) -> Self {
        let this = Self {
            inner: RwLock::new(SharedBotRegistry {
                bot_accounts: HashMap::new(),
                bot_usernames: HashMap::new(),
            }),
        };
        for bot in bot_repository.get_bots() {
            this.register_bot(&bot.account_id, &bot.username, &bot.display_name);
        }
        this
    }

    fn register_bot(
        &self,
        account_id: &AccountId,
        username: &str,
        display_name: &str,
    ) -> Option<Account> {
        let mut registry = self.inner.write().unwrap();

        if registry.bot_accounts.contains_key(&account_id) {
            return None;
        }

        let account = Account::new(
            account_id.clone(),
            AccountType::Bot,
            AccountRole::User,
            ModerationFlags::new(),
            username.to_string(),
            display_name.to_string(),
            None,
        );
        registry
            .bot_accounts
            .insert(account_id.clone(), account.clone());
        registry
            .bot_usernames
            .insert(account.username.clone(), account.account_id.clone());
        Some(account)
    }

    pub fn get_by_username(&self, username: &str) -> Option<Account> {
        let registry = self.inner.read().unwrap();
        if let Some(account_id) = registry.bot_usernames.get(username) {
            if let Some(bot) = registry.bot_accounts.get(account_id) {
                return Some(bot.clone());
            }
        }
        None
    }

    pub fn get_by_id(&self, account_id: &AccountId) -> Option<Account> {
        let registry = self.inner.read().unwrap();
        if let Some(bot) = registry.bot_accounts.get(&account_id) {
            return Some(bot.clone());
        }
        None
    }

    pub fn update_bot<R>(
        &self,
        account_id: &AccountId,
        update: impl FnOnce(&mut Account) -> R,
    ) -> Option<R> {
        let mut registry = self.inner.write().unwrap();
        if let Some(bot) = registry.bot_accounts.get_mut(account_id) {
            Some(update(bot))
        } else {
            None
        }
    }
}
