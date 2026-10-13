use std::time::Duration;

use crate::domain::UserId;

#[async_trait::async_trait]
pub trait AccountPort {
    async fn register_account(
        &self,
        email: &str,
        username: &str,
        password: &str,
    ) -> Result<UserId, String>;
    async fn login_account(
        &self,
        identifier: &str,
        password: &str,
    ) -> Result<Option<UserId>, String>;
    async fn create_guest_account(&self) -> Result<UserId, String>;
    async fn find_account(&self, identifier: &str) -> Result<Option<UserId>, String>;
    async fn reset_account_password(
        &self,
        user_id: UserId,
        token: &str,
        new_password: &str,
    ) -> Result<Option<()>, String>;

    fn generate_reset_token(&self, user_id: UserId, duration: Duration) -> Result<String, String>;
    async fn get_account_info(&self, user_id: UserId) -> Result<Option<AccountInfo>, ()>;
}

#[derive(Clone, Debug)]
pub struct AccountInfo {
    pub username: String,
    pub email: Option<String>,
}
