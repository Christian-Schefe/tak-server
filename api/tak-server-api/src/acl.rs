use std::sync::Arc;

use passwords::PasswordGenerator;
use tak_auth_ory::OryAuthenticationService;
use tak_server_app::{
    Application,
    domain::{AccountId, PlayerId},
    ports::{authentication::Account, email::EmailPort},
};

pub struct LegacyAPIAntiCorruptionLayer {
    app: Arc<Application>,
    auth: Arc<OryAuthenticationService>,
    email_port: Arc<dyn EmailPort + Send + Sync + 'static>,
}

impl LegacyAPIAntiCorruptionLayer {
    pub fn new(
        app: Arc<Application>,
        auth: Arc<OryAuthenticationService>,
        email_port: Arc<dyn EmailPort + Send + Sync + 'static>,
    ) -> Self {
        Self {
            app,
            auth,
            email_port,
        }
    }

    pub async fn get_player_id_by_username(&self, username: &str) -> Option<PlayerId> {
        let account = self.auth.find_by_username(username).await?;
        let player_id = self
            .app
            .player_resolver_service
            .resolve_player_id_by_account_id(&account.account_id)
            .await
            .ok()?;
        Some(player_id)
    }

    pub async fn login_username_password(
        &self,
        username: &str,
        password: &str,
    ) -> Result<Account, String> {
        self.auth.login_username_password(username, password).await
    }

    pub fn generate_random_password() -> String {
        uuid::Uuid::new_v4().to_string()
    }

    fn generate_temporary_password() -> String {
        let password_gen = PasswordGenerator::new()
            .length(8)
            .numbers(true)
            .lowercase_letters(true)
            .uppercase_letters(false)
            .spaces(false)
            .symbols(false)
            .exclude_similar_characters(true)
            .strict(true);
        password_gen.generate_one().unwrap()
    }

    fn send_password_email(
        &self,
        to: &str,
        username: &str,
        temp_password: &str,
    ) -> Result<(), String> {
        let subject = "Welcome to Playtak!";
        let body = format!(
            "Hello {},\n\n\
        Your account has been created successfully!\n\n\
        Here are your login details:\n\
        Username: {}\n\
        Temporary Password: {}\n\n\
        Please log in and change your password as soon as possible.\n\n\
        Best regards,\n\
        The Playtak Team",
            username, username, temp_password
        );
        self.email_port.send_email(to, &subject, &body)?;
        Ok(())
    }

    pub async fn register_username_email(
        &self,
        username: &str,
        email: &str,
    ) -> Result<Account, String> {
        //let password = Self::generate_temporary_password();
        let password = "changeme".to_string();

        let password_hash =
            bcrypt::hash(password.clone(), bcrypt::DEFAULT_COST).map_err(|e| e.to_string())?;
        let account = self
            .auth
            .create_account(username, email, &password_hash)
            .await
            .map_err(|e| format!("Failed to create account: {}", e))?;

        // self.send_password_email(email, username, &password)?;
        Ok(account)
    }

    pub async fn request_password_reset(
        &self,
        _username: &str,
        _email: &str,
    ) -> Result<(), String> {
        Err("Not implemented".to_string())
    }

    pub async fn reset_password(
        &self,
        _username: &str,
        _temp_password: &str,
        _new_password: &str,
    ) -> Result<(), String> {
        Err("Not implemented".to_string())
    }

    pub async fn change_password(
        &self,
        _account_id: AccountId,
        _old_password: &str,
        _new_password: &str,
    ) -> Result<(), String> {
        Err("Not implemented".to_string())
    }
}
