use crate::ports::authentication::ClientId;

pub trait ContactRepository {
    fn store_email_contact(&self, client_id: &ClientId, email: &str)
    -> Result<(), StoreEmailError>;
    fn remove_email_contact(&self, client_id: &ClientId) -> bool;
    fn get_email_contact(&self, client_id: &ClientId) -> Option<String>;
}

pub enum StoreEmailError {
    InvalidEmail,
    StorageError,
}
