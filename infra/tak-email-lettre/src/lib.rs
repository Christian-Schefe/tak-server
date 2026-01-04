use std::str::FromStr;

use lettre::{Message, SmtpTransport, Transport, message::Mailbox};
use tak_server_app::ports::email::EmailPort;

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
    fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<(), String> {
        let email = Message::builder()
            .from(self.from.clone())
            .to(Mailbox::from_str(to).map_err(|e| format!("Invalid to address: {}", e))?)
            .subject(subject)
            .body(body.to_string())
            .map_err(|e| format!("Failed to build email: {}", e))?;
        self.transport
            .send(&email)
            .map_err(|e| format!("Failed to send email: {}", e))?;
        Ok(())
    }
}

pub struct LogEmailAdapter;

impl EmailPort for LogEmailAdapter {
    fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<(), String> {
        log::info!(
            "Simulated sending email to: {}, subject: {}, body: {}",
            to,
            subject,
            body
        );
        Ok(())
    }
}
