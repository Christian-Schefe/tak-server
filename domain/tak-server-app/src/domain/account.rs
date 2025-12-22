use crate::{domain::PlayerId, ports::authentication::ClientId};

use rustrict::CensorStr;
use uuid::Uuid;

pub trait AccountRepository {
    fn set_player_silenced(&self, player_id: PlayerId, silenced: bool);
    fn set_player_banned(&self, player_id: PlayerId, banned: bool);
    fn set_player_role(&self, player_id: PlayerId, role: AccountRole);
    fn set_player_is_bot(&self, player_id: PlayerId, is_bot: bool);
    fn get_account_by_client_id(&self, client_id: ClientId) -> Option<Account>;
    fn get_account(&self, player_id: PlayerId) -> Option<Account>;
    fn get_account_by_username(&self, username: &str) -> Option<Account>;
    fn create_account(&self, account: Account) -> Result<(), CreateAccountRepoError>;
}

pub enum CreateAccountRepoError {
    UsernameTaken,
    StorageError,
}

pub struct Account {
    pub player_id: PlayerId,
    pub username: String,
    pub is_silenced: bool,
    pub is_banned: bool,
    pub role: AccountRole,
    pub is_bot: bool,
    pub is_guest: bool,
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
}

pub struct AccountFactoryImpl;

impl AccountFactoryImpl {
    pub fn new() -> Self {
        Self {}
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
        let player_id = PlayerId(Uuid::new_v4());
        let acc = Account {
            player_id,
            username: username.to_string(),
            is_silenced: false,
            is_banned: false,
            role: AccountRole::User,
            is_bot: false,
            is_guest: false,
        };
        Ok(acc)
    }
}
