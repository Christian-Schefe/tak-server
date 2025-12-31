use crate::domain::{
    AccountId,
    moderation::{AccountRole, ModerationFlag, ModerationFlags},
};

#[async_trait::async_trait]
pub trait AuthenticationPort {
    async fn get_or_create_guest_account(&self, token: &str) -> Account;
    async fn get_account(&self, account_id: &AccountId) -> Option<Account>;
    async fn set_role(&self, account_id: &AccountId, role: AccountRole) -> Result<(), ()>;
    async fn add_flag(&self, account_id: &AccountId, flag: ModerationFlag) -> Result<(), ()>;
    async fn remove_flag(&self, account_id: &AccountId, flag: ModerationFlag) -> Result<(), ()>;
}

#[derive(Clone, Debug)]
pub struct Account {
    pub account_id: AccountId,
    pub account_type: AccountType,
    pub role: AccountRole,
    pub flags: ModerationFlags,
    pub username: String,
    pub display_name: String,
    pub email: Option<String>,
}

impl Account {
    pub fn new(
        account_id: AccountId,
        account_type: AccountType,
        role: AccountRole,
        flags: ModerationFlags,
        username: String,
        display_name: String,
        email: Option<String>,
    ) -> Self {
        Self {
            account_id,
            account_type,
            role,
            flags,
            username,
            display_name,
            email,
        }
    }

    pub fn is_bot(&self) -> bool {
        matches!(self.account_type, AccountType::Bot)
    }

    pub fn is_flagged(&self, flag: ModerationFlag) -> bool {
        self.flags.is_flagged(flag)
    }

    pub fn add_flag(&mut self, flag: ModerationFlag) {
        self.flags.set_flag(flag);
    }

    pub fn remove_flag(&mut self, flag: ModerationFlag) {
        self.flags.unset_flag(flag);
    }

    pub fn get_username(&self) -> &str {
        &self.username
    }

    pub fn get_display_name(&self) -> &str {
        &self.display_name
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AccountType {
    Player,
    Guest,
    Bot,
}
