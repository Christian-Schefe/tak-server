use crate::{
    ServiceResult,
    client::ClientId,
    protocol::json::{ClientResponse, ProtocolJsonHandler},
};

impl ProtocolJsonHandler {
    pub fn handle_login_message(
        &self,
        id: &ClientId,
        token: &str,
    ) -> ServiceResult<ClientResponse> {
        self.player_service.try_login_jwt(id, &token)?;
        Ok(ClientResponse::Ok)
    }

    pub fn handle_login_guest_message(
        &self,
        id: &ClientId,
        token: Option<&str>,
    ) -> ServiceResult<ClientResponse> {
        self.player_service.try_login_guest(id, token)?;
        Ok(ClientResponse::Ok)
    }
}
