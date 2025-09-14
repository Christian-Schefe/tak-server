use lettre::{
    Address, Message, SmtpTransport, Transport, message::Mailbox,
    transport::smtp::authentication::Credentials,
};

pub fn send_email(to: &str, subject: &str, body: &str) -> Result<(), String> {
    let host = std::env::var("TAK_EMAIL_HOST").map_err(|_| "TAK_EMAIL_HOST env var not set")?;
    let user = std::env::var("TAK_EMAIL_USER").map_err(|_| "TAK_EMAIL_USER env var not set")?;
    let password =
        std::env::var("TAK_EMAIL_PASSWORD").map_err(|_| "TAK_EMAIL_PASSWORD env var not set")?;
    let from = std::env::var("TAK_EMAIL_FROM").map_err(|_| "TAK_EMAIL_FROM env var not set")?;
    let email = Message::builder()
        .from(Mailbox::new(
            None,
            Address::try_from(from.to_string())
                .map_err(|e| format!("Invalid from address: {}", e))?,
        ))
        .to(Mailbox::new(
            None,
            Address::try_from(to.to_string()).map_err(|e| format!("Invalid to address: {}", e))?,
        ))
        .subject(subject)
        .body(body.to_string())
        .map_err(|e| format!("Failed to build email: {}", e))?;

    let transport = SmtpTransport::relay(&host)
        .map_err(|e| format!("Failed to create SMTP transport: {}", e))?
        .credentials(Credentials::new(user, password))
        .build();
    transport
        .send(&email)
        .map_err(|e| format!("Failed to send email: {}", e))?;
    Ok(())
}
