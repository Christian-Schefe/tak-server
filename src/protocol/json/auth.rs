use crate::{client::ClientId, protocol::json::ProtocolJsonHandler};

impl ProtocolJsonHandler {
    pub fn handle_login_message(&self, id: &ClientId, token: &str) {
        if let Err(e) = self.player_service.try_login_jwt(id, &token) {
            eprintln!("Login with token failed for user {}: {}", id, e);
        } else {
            println!("User {} logged in successfully", id);
        }
    }

    pub fn handle_login_guest_message(&self, id: &ClientId, token: Option<&str>) {
        if let Err(e) = self.player_service.try_login_guest(id, token) {
            eprintln!("Guest login failed for user {}: {}", id, e);
        }
    }
}
