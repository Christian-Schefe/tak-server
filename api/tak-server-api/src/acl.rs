use std::sync::Arc;

use passwords::PasswordGenerator;
use tak_server_app::{
    Application,
    domain::{AccountId, PlayerId},
    ports::{authentication::Account, email::EmailPort},
};

pub struct LegacyAPIAntiCorruptionLayer {
    app: Arc<Application>,
    email_port: Arc<dyn EmailPort + Send + Sync + 'static>,
}

impl LegacyAPIAntiCorruptionLayer {
    pub fn new(
        app: Arc<Application>,
        email_port: Arc<dyn EmailPort + Send + Sync + 'static>,
    ) -> Self {
        Self { app, email_port }
    }

    pub async fn get_player_id_by_username(&self, username: &str) -> Option<PlayerId> {
        todo!();
    }

    pub async fn login_username_password(
        &self,
        username: &str,
        password: &str,
    ) -> Result<Account, String> {
        todo!();
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
        let password = Self::generate_temporary_password();
        self.send_password_email(email, username, &password);
        todo!();
    }

    pub async fn request_password_reset(&self, username: &str, email: &str) -> Result<(), String> {
        let temp_password = Self::generate_temporary_password();
        self.send_password_email(email, username, &temp_password);
        todo!();
    }

    pub async fn reset_password(
        &self,
        username: &str,
        temp_password: &str,
        new_password: &str,
    ) -> Result<(), String> {
        todo!();
    }

    pub async fn change_password(
        &self,
        account_id: AccountId,
        old_password: &str,
        new_password: &str,
    ) -> Result<(), String> {
        todo!();
    }
}
