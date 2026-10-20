use std::time::Duration;

use tak_domain::UserId;

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
    fn generate_refresh_token(
        &self,
        user_id: UserId,
        duration: Duration,
    ) -> Result<RefreshToken, String>;
    fn rotate_refresh_token(
        &self,
        user_id: UserId,
        refresh_token: &str,
        duration: Duration,
    ) -> Result<Option<RefreshToken>, String>;
    fn invalidate_refresh_token(&self, user_id: UserId) -> Result<(), String>;
    async fn get_account_info(&self, user_id: UserId) -> Result<Option<AccountInfo>, ()>;
}

#[derive(Clone, Debug)]
pub struct RefreshToken {
    pub token: String,
    pub valid_until: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug)]
pub struct AccountInfo {
    pub username: String,
    pub email: Option<String>,
}
