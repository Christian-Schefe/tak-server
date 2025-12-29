use std::collections::HashSet;

use crate::domain::{
    AccountId,
    moderation::{AccountRole, ModerationFlag},
};

#[async_trait::async_trait]
pub trait AuthenticationPort {
    async fn get_or_create_guest_account(&self, token: &str) -> Account;
    async fn get_account(&self, account_id: &AccountId) -> Option<Account>;
    async fn set_role(&self, account_id: &AccountId, role: AccountRole) -> Result<(), ()>;
    async fn add_flag(&self, account_id: &AccountId, flag: ModerationFlag) -> Result<(), ()>;
    async fn remove_flag(&self, account_id: &AccountId, flag: ModerationFlag) -> Result<(), ()>;
    async fn query_accounts(&self, query: AccountQuery) -> Vec<Account>;
}

pub struct AccountQuery {
    pub flag: Option<ModerationFlag>,
    pub role: Option<AccountRole>,
    pub account_type: Option<AccountType>,
}

impl AccountQuery {
    pub fn new() -> Self {
        Self {
            flag: None,
            role: None,
            account_type: None,
        }
    }

    pub fn with_flag(mut self, flag: ModerationFlag) -> Self {
        self.flag = Some(flag);
        self
    }

    pub fn with_role(mut self, role: AccountRole) -> Self {
        self.role = Some(role);
        self
    }

    pub fn with_account_type(mut self, account_type: AccountType) -> Self {
        self.account_type = Some(account_type);
        self
    }
}

#[derive(Clone, Debug)]
pub struct Account {
    pub account_id: AccountId,
    pub account_type: AccountType,
    pub role: AccountRole,
    flags: HashSet<ModerationFlag>,
    pub username: String,
    pub email: Option<String>,
}

impl Account {
    pub fn new(
        account_id: AccountId,
        account_type: AccountType,
        role: AccountRole,
        username: String,
        email: Option<String>,
    ) -> Self {
        Self {
            account_id,
            account_type,
            role,
            flags: HashSet::new(),
            username,
            email,
        }
    }

    pub fn is_bot(&self) -> bool {
        matches!(self.account_type, AccountType::Bot)
    }

    pub fn add_flag(&mut self, flag: ModerationFlag) -> bool {
        self.flags.insert(flag)
    }

    pub fn remove_flag(&mut self, flag: ModerationFlag) -> bool {
        self.flags.remove(&flag)
    }

    pub fn is_flagged(&self, flag: ModerationFlag) -> bool {
        self.flags.contains(&flag)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AccountType {
    Player,
    Guest,
    Bot,
}
