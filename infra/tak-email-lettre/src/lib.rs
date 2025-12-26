use std::str::FromStr;

use lettre::{
    Message, SmtpTransport, Transport, message::Mailbox,
    transport::smtp::authentication::Credentials,
};
use tak_server_app::ports::email::EmailPort;

pub struct LettreEmailAdapter {
    transport: SmtpTransport,
    from: Mailbox,
}

impl LettreEmailAdapter {
    pub fn new() -> Self {
        let host = std::env::var("TAK_EMAIL_HOST").expect("TAK_EMAIL_HOST env var not set");
        let user = std::env::var("TAK_EMAIL_USER").expect("TAK_EMAIL_USER env var not set");
        let password =
            std::env::var("TAK_EMAIL_PASSWORD").expect("TAK_EMAIL_PASSWORD env var not set");
        let from = std::env::var("TAK_EMAIL_FROM").expect("TAK_EMAIL_FROM env var not set");
        let from = Mailbox::from_str(&from).expect("Invalid TAK_EMAIL_FROM address");
        let transport = SmtpTransport::relay(&host)
            .expect("Failed to create SMTP transport")
            .credentials(Credentials::new(user.clone(), password.clone()))
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
