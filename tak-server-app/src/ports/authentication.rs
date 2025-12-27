use std::collections::HashSet;

use crate::domain::{
    AccountId,
    account::{AccountFlag, AccountRole},
};

#[async_trait::async_trait]
pub trait AuthenticationPort {
    async fn get_account(&self, account_id: AccountId) -> Option<Account>;
    async fn set_role(&self, account_id: AccountId, role: AccountRole) -> Result<(), ()>;
    async fn add_flag(&self, account_id: AccountId, flag: AccountFlag) -> Result<bool, ()>;
    async fn remove_flag(&self, account_id: AccountId, flag: AccountFlag) -> Result<bool, ()>;
    async fn query_accounts(&self, query: AccountQuery) -> Vec<Account>;
}

pub struct AccountQuery {
    pub flag: Option<AccountFlag>,
    pub role: Option<AccountRole>,
}

impl AccountQuery {
    pub fn new() -> Self {
        Self {
            flag: None,
            role: None,
        }
    }

    pub fn with_flag(mut self, flag: AccountFlag) -> Self {
        self.flag = Some(flag);
        self
    }

    pub fn with_role(mut self, role: AccountRole) -> Self {
        self.role = Some(role);
        self
    }
}

pub struct Account {
    pub account_id: AccountId,
    pub subject_type: AuthSubject,
    pub role: AccountRole,
    pub flags: HashSet<AccountFlag>,
}

pub enum AuthSubject {
    Player {
        username: String,
        email: Option<String>,
    },
    Guest {
        guest_number: u64,
    },
}
