use std::sync::Arc;

use tak_server_app::{
    domain::{
        AccountId, RepoError, RepoRetrieveError,
        moderation::{AccountRole, ModerationFlags},
    },
    ports::authentication::{Account, AccountType},
};

#[async_trait::async_trait]
pub trait GuestRepository {
    async fn get_account(&self, account_id: &AccountId) -> Result<GuestAccount, RepoRetrieveError>;
    async fn get_by_guest_number(
        &self,
        guest_number: i64,
    ) -> Result<GuestAccount, RepoRetrieveError>;
    async fn create_guest(&self) -> Result<GuestAccount, RepoError>;
    async fn update_guest(&self, account: &GuestAccount) -> Result<(), RepoRetrieveError>;
}

pub struct GuestAccount {
    pub account_id: AccountId,
    pub guest_number: i64,
    pub flags: ModerationFlags,
}

impl GuestAccount {
    fn to_account(&self) -> Account {
        Account {
            account_id: self.account_id.clone(),
            username: format!("guest{}", self.guest_number),
            account_type: AccountType::Guest,
            role: AccountRole::User,
            flags: self.flags.clone(),
            display_name: format!("Guest {}", self.guest_number),
            email: None,
        }
    }
    fn update_from_account(&mut self, account: &Account) {
        self.flags = account.flags.clone();
    }
}

pub struct GuestService {
    guest_repository: Arc<dyn GuestRepository + Send + Sync + 'static>,
}

impl GuestService {
    pub fn new(guest_repository: Arc<dyn GuestRepository + Send + Sync + 'static>) -> Self {
        Self { guest_repository }
    }

    pub async fn create_guest(&self) -> Option<Account> {
        match self.guest_repository.create_guest().await {
            Ok(guest_account) => Some(guest_account.to_account()),
            Err(RepoError::StorageError(e)) => {
                tracing::error!("Failed to create guest account: {}", e);
                None
            }
        }
    }

    pub async fn get_by_username(&self, username: &str) -> Option<Account> {
        if let Some(guest_number) = username
            .strip_prefix("guest")
            .and_then(|num| num.parse::<i64>().ok())
        {
            match self
                .guest_repository
                .get_by_guest_number(guest_number)
                .await
            {
                Ok(guest_account) => return Some(guest_account.to_account()),
                Err(RepoRetrieveError::NotFound) => return None,
                Err(RepoRetrieveError::StorageError(e)) => {
                    tracing::error!(
                        "Failed to retrieve guest account by guest number {}: {}",
                        guest_number,
                        e
                    );
                    return None;
                }
            }
        }
        None
    }

    pub async fn get_by_account_id(&self, account_id: &AccountId) -> Option<Account> {
        match self.guest_repository.get_account(account_id).await {
            Ok(guest_account) => Some(guest_account.to_account()),
            Err(RepoRetrieveError::NotFound) => None,
            Err(RepoRetrieveError::StorageError(e)) => {
                tracing::error!("Failed to retrieve guest account {}: {}", account_id, e);
                None
            }
        }
    }

    pub async fn update_guest<R>(
        &self,
        account_id: &AccountId,
        update: impl FnOnce(&mut Account) -> R,
    ) -> Option<R> {
        if let Ok(mut guest_account) = self.guest_repository.get_account(account_id).await {
            let mut account = guest_account.to_account();
            let res = update(&mut account);
            guest_account.update_from_account(&account);
            if self
                .guest_repository
                .update_guest(&guest_account)
                .await
                .is_ok()
            {
                Some(res)
            } else {
                None
            }
        } else {
            None
        }
    }
}
