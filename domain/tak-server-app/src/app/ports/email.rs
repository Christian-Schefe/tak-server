pub trait EmailPort {
    fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<(), String>;
}
