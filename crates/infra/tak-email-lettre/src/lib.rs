use std::str::FromStr;

use lettre::{Message, SmtpTransport, Transport, message::Mailbox};
use tak_server_app::{
    errors::{AppResult, InternalError, InvalidEmailAddress, OneOf},
    ports::email::EmailPort,
};

pub struct LettreEmailAdapter {
    transport: SmtpTransport,
    from: Mailbox,
}

impl LettreEmailAdapter {
    pub fn new() -> Self {
        let url = std::env::var("TAK_EMAIL_URL").expect("TAK_EMAIL_URL env var not set");
        let from = std::env::var("TAK_EMAIL_FROM").expect("TAK_EMAIL_FROM env var not set");
        let from = Mailbox::from_str(&from).expect("Invalid TAK_EMAIL_FROM address");
        let transport = SmtpTransport::from_url(&url)
            .expect("Invalid smtp transport url")
            .build();
        Self { transport, from }
    }
}

impl EmailPort for LettreEmailAdapter {
    fn send_email(
        &self,
        to: &str,
        subject: &str,
        body: &str,
    ) -> AppResult<(), (InvalidEmailAddress, InternalError)> {
        let email = Message::builder()
            .from(self.from.clone())
            .to(Mailbox::from_str(to).map_err(|_| OneOf::new(InvalidEmailAddress))?)
            .subject(subject)
            .body(body.to_string())
            .map_err(|e| {
                tracing::error!("Failed to build email message: {}", e);
                OneOf::new(InternalError)
            })?;
        self.transport.send(&email).map_err(|e| {
            tracing::error!("Failed to send email: {}", e);
            OneOf::new(InternalError)
        })?;
        Ok(())
    }
}
