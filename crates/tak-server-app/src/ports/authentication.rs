use std::time::Duration;

use tak_domain::UserId;

use crate::errors::{AppResult, EmailTaken, InternalError, InvalidCredentials};

#[async_trait::async_trait]
pub trait AccountPort {
    async fn register_account(
        &self,
        email: &str,
        username: &str,
        password: &str,
    ) -> AppResult<UserId, (EmailTaken, InternalError)>;
    async fn login_account(
        &self,
        identifier: &str,
        password: &str,
    ) -> AppResult<UserId, (InvalidCredentials, InternalError)>;
    async fn create_guest_account(&self) -> AppResult<UserId, (InternalError,)>;
    async fn find_account(&self, identifier: &str) -> AppResult<Option<UserId>, (InternalError,)>;
    async fn reset_account_password(
        &self,
        user_id: UserId,
        token: &str,
        new_password: &str,
    ) -> AppResult<Option<()>, (InvalidCredentials, InternalError)>;

    fn generate_reset_token(
        &self,
        user_id: UserId,
        duration: Duration,
    ) -> AppResult<String, (InternalError,)>;
    fn generate_refresh_token(
        &self,
        user_id: UserId,
        duration: Duration,
    ) -> AppResult<RefreshToken, (InternalError,)>;
    fn rotate_refresh_token(
        &self,
        user_id: UserId,
        refresh_token: &str,
        duration: Duration,
    ) -> AppResult<RefreshToken, (InvalidCredentials, InternalError)>;
    fn invalidate_refresh_token(&self, user_id: UserId) -> AppResult<(), (InternalError,)>;
    async fn get_account_info(
        &self,
        user_id: UserId,
    ) -> AppResult<Option<AccountInfo>, (InternalError,)>;
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
