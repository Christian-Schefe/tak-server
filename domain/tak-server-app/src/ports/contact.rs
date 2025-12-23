use crate::ports::authentication::SubjectId;

pub trait ContactRepository {
    fn store_email_contact(
        &self,
        subject_id: &SubjectId,
        email: &str,
    ) -> Result<(), StoreEmailError>;
    fn remove_email_contact(&self, subject_id: &SubjectId) -> bool;
    fn get_email_contact(&self, subject_id: &SubjectId) -> Option<String>;
}

pub enum StoreEmailError {
    InvalidEmail,
    StorageError,
}
