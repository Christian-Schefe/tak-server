pub trait EmailPort {
    fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<(), SendEmailError>;
}

#[derive(Debug, Clone)]
pub enum SendEmailError {
    InvalidToAddress(String),
    SendEmailError(String),
}
