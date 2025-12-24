use std::time::Duration;

use crate::app::domain::AccountId;

pub trait AuthenticationService {
    fn register_username_password(
        &self,
        subject_id: &SubjectId,
        username: &str,
        password: &str,
    ) -> Result<SubjectId, AuthRegisterError>;
    fn issue_session_token(&self, subject_id: &SubjectId, ttl: Duration) -> SessionToken;

    fn authenticate_username_password(
        &self,
        username: &str,
        password: &str,
    ) -> Result<SubjectId, AuthError>;
    fn authenticate_session_token(&self, token: &SessionToken) -> Result<SubjectId, AuthError>;

    fn revoke_credentials(&self, subject_id: &SubjectId);
    fn revoke_sessions(&self, subject_id: &SubjectId);
}

pub enum AuthRegisterError {
    IdentifierTaken,
    StorageError,
}

pub enum AuthError {
    InvalidCredentials,
}

pub struct SessionToken(pub String);

pub enum SubjectId {
    Account(AccountId),
}
