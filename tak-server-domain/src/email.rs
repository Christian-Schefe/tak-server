use std::{str::FromStr, sync::Arc};

use lettre::{
    Message, SmtpTransport, Transport, message::Mailbox,
    transport::smtp::authentication::Credentials,
};

use crate::{ServiceError, ServiceResult};

pub type ArcEmailService = Arc<Box<dyn EmailService + Send + Sync>>;
pub trait EmailService {
    fn send_email(&self, to: &str, subject: &str, body: &str) -> ServiceResult<()>;
}

pub struct EmailServiceImpl;

impl EmailService for EmailServiceImpl {
    fn send_email(&self, to: &str, subject: &str, body: &str) -> ServiceResult<()> {
        let host = std::env::var("TAK_EMAIL_HOST")
            .map_err(|_| ServiceError::Internal("TAK_EMAIL_HOST env var not set".into()))?;
        let user = std::env::var("TAK_EMAIL_USER")
            .map_err(|_| ServiceError::Internal("TAK_EMAIL_USER env var not set".into()))?;
        let password = std::env::var("TAK_EMAIL_PASSWORD")
            .map_err(|_| ServiceError::Internal("TAK_EMAIL_PASSWORD env var not set".into()))?;
        let from = std::env::var("TAK_EMAIL_FROM")
            .map_err(|_| ServiceError::Internal("TAK_EMAIL_FROM env var not set".into()))?;
        let email = Message::builder()
            .from(
                Mailbox::from_str(&from)
                    .map_err(|e| ServiceError::Internal(format!("Invalid from address: {}", e)))?,
            )
            .to(Mailbox::from_str(to)
                .map_err(|e| ServiceError::Internal(format!("Invalid to address: {}", e)))?)
            .subject(subject)
            .body(body.to_string())
            .map_err(|e| ServiceError::Internal(format!("Failed to build email: {}", e)))?;

        let transport = SmtpTransport::relay(&host)
            .map_err(|e| ServiceError::Internal(format!("Failed to create SMTP transport: {}", e)))?
            .credentials(Credentials::new(user, password))
            .build();
        transport
            .send(&email)
            .map_err(|e| ServiceError::Internal(format!("Failed to send email: {}", e)))?;
        Ok(())
    }
}
