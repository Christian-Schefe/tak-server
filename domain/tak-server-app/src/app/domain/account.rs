use std::sync::{Arc, Mutex};

use crate::app::domain::AccountId;

use rustrict::CensorStr;
use uuid::Uuid;

pub trait AccountRepository {
    fn set_account_role(&self, account_id: AccountId, role: AccountRole);
    fn get_account(&self, account_id: AccountId) -> Option<Account>;
    fn get_account_by_username(&self, username: &str) -> Option<Account>;
    fn create_account(&self, account: Account) -> Result<(), CreateAccountRepoError>;
}

pub struct Account {
    pub account_id: AccountId,
    pub username: String,
    pub role: AccountRole,
    pub is_guest: bool,
}

pub enum CreateAccountRepoError {
    UsernameTaken,
    StorageError,
}

pub enum AccountRole {
    User,
    Moderator,
    Admin,
}

pub enum CreateAccountError {
    InvalidUsername,
}

pub trait AccountFactory {
    fn create_account(&self, username: &str) -> Result<Account, CreateAccountError>;
    fn create_guest_account(&self) -> Account;
}

pub struct AccountFactoryImpl {
    next_guest_number: Arc<Mutex<u64>>,
}

impl AccountFactoryImpl {
    pub fn new() -> Self {
        Self {
            next_guest_number: Arc::new(Mutex::new(0)),
        }
    }

    fn get_next_guest_number(&self) -> u64 {
        let mut guard = self.next_guest_number.lock().unwrap();
        let number = *guard;
        *guard += 1;
        number
    }
}

impl AccountFactory for AccountFactoryImpl {
    fn create_account(&self, username: &str) -> Result<Account, CreateAccountError> {
        if !username
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
            || username.len() < 3
            || username.len() > 16
        {
            return Err(CreateAccountError::InvalidUsername);
        }
        if username.is_inappropriate() {
            return Err(CreateAccountError::InvalidUsername);
        }
        if username.starts_with("Guest") {
            return Err(CreateAccountError::InvalidUsername);
        }
        let account_id = AccountId(Uuid::new_v4());
        let acc = Account {
            account_id,
            username: username.to_string(),
            role: AccountRole::User,
            is_guest: false,
        };
        Ok(acc)
    }

    fn create_guest_account(&self) -> Account {
        let account_id = AccountId(Uuid::new_v4());
        let guest_number = self.get_next_guest_number();
        let username = format!("Guest{}", guest_number);
        Account {
            account_id,
            username,
            role: AccountRole::User,
            is_guest: true,
        }
    }
}

pub trait PermissionPolicy {
    fn has_permissions(&self, requester: &Account, target: &Account) -> bool;
}

pub struct AdminAccountPolicy;

impl PermissionPolicy for AdminAccountPolicy {
    fn has_permissions(&self, requester: &Account, target: &Account) -> bool {
        matches!(requester.role, AccountRole::Admin) && !matches!(target.role, AccountRole::Admin)
    }
}

pub struct ModeratorAccountPolicy;

impl PermissionPolicy for ModeratorAccountPolicy {
    fn has_permissions(&self, requester: &Account, target: &Account) -> bool {
        matches!(requester.role, AccountRole::Admin | AccountRole::Moderator)
            && !matches!(target.role, AccountRole::Admin | AccountRole::Moderator)
    }
}
