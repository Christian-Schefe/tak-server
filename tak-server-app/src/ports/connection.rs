use crate::domain::{AccountId, ListenerId};

#[async_trait::async_trait]
pub trait AccountConnectionPort {
    async fn get_connection_id(&self, account_id: &AccountId) -> Option<ListenerId>;
}

pub trait AccountOnlineStatusPort {
    fn set_account_online(&self, account_id: &AccountId) -> Option<Vec<AccountId>>;
    fn set_account_offline(&self, account_id: &AccountId) -> Option<Vec<AccountId>>;
    fn get_online_accounts(&self) -> Vec<AccountId>;
}
