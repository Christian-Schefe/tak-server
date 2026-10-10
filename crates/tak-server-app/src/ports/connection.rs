use crate::domain::{UserId, ListenerId};

#[async_trait::async_trait]
pub trait AccountConnectionPort {
    async fn get_connection_id(&self, user_id: UserId) -> Option<ListenerId>;
}

pub trait AccountOnlineStatusPort {
    fn set_account_online(&self, user_id: UserId) -> Option<Vec<UserId>>;
    fn set_account_offline(&self, user_id: UserId) -> Option<Vec<UserId>>;
    fn get_online_accounts(&self) -> Vec<UserId>;
}
