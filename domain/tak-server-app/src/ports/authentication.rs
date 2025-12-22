pub trait AuthenticationService {
    fn authenticate(&self, client_id: &ClientId, client_secret: &str) -> bool;
    fn register_credentials(
        &self,
        client_id: &ClientId,
        client_secret: &str,
    ) -> Result<(), AuthRegisterError>;
    fn remove_credentials(&self, client_id: &ClientId) -> bool;
}

pub enum AuthRegisterError {
    IdentifierTaken,
    StorageError,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ClientId(String);

impl ClientId {
    pub fn new(id: &str) -> Self {
        Self(id.to_string())
    }
}
