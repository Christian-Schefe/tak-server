use crate::errors::{AppResult, InternalError, InvalidEmailAddress};

pub trait EmailPort {
    fn send_email(
        &self,
        to: &str,
        subject: &str,
        body: &str,
    ) -> AppResult<(), (InvalidEmailAddress, InternalError)>;
}
