use crate::domain::{
    UserId,
    moderation::{AccountRole, ModerationFlag, ModerationFlags},
};

#[async_trait::async_trait]
pub trait AuthenticationPort {
    async fn get_account(&self, user_id: UserId) -> Result<Option<Account>, ()>;
    async fn set_role(&self, user_id: UserId, role: AccountRole) -> Result<(), ()>;
    async fn add_flag(&self, user_id: UserId, flag: ModerationFlag) -> Result<(), ()>;
    async fn remove_flag(&self, user_id: UserId, flag: ModerationFlag) -> Result<(), ()>;
}

#[derive(Clone, Debug)]
pub struct Account {
    pub account_type: AccountType,
    pub role: AccountRole,
    pub flags: ModerationFlags,
    pub username: String,
    pub email: Option<String>,
}

impl Account {
    pub fn new(
        account_type: AccountType,
        role: AccountRole,
        flags: ModerationFlags,
        username: String,
        email: Option<String>,
    ) -> Self {
        Self {
            account_type,
            role,
            flags,
            username,
            email,
        }
    }

    pub fn is_bot(&self) -> bool {
        matches!(self.account_type, AccountType::Bot)
    }

    pub fn is_admin(&self) -> bool {
        matches!(self.role, AccountRole::Admin)
    }

    pub fn is_guest(&self) -> bool {
        matches!(self.account_type, AccountType::Guest)
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AccountType {
    Player,
    Guest,
    Bot,
}
