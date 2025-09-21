use crate::{
    ServiceError,
    client::ClientId,
    player::PlayerUsername,
    protocol::v2::{ProtocolV2Handler, ProtocolV2Result},
};

impl ProtocolV2Handler {
    pub fn handle_login_message(&self, id: &ClientId, parts: &[&str]) -> ProtocolV2Result {
        if parts.len() >= 2 && parts[1] == "Guest" {
            let token = parts.get(2).copied();
            let username = self.player_service.try_login_guest(id, token)?;
            return Ok(Some(format!("Welcome {}!", username)));
        }
        if parts.len() != 3 {
            return ServiceError::bad_request("Invalid Login message format");
        }
        let username = parts[1].to_string();
        let password = parts[2].to_string();

        self.player_service.try_login(id, &username, &password)?;
        Ok(Some(format!("Welcome {}!", username)))
    }

    pub fn handle_login_token_message(&self, id: &ClientId, parts: &[&str]) -> ProtocolV2Result {
        if parts.len() != 2 {
            return ServiceError::bad_request("Invalid LoginToken message format");
        }
        let token = parts[1];
        let username = self.player_service.try_login_jwt(id, token)?;
        Ok(Some(format!("Welcome {}!", username)))
    }

    pub fn handle_register_message(&self, parts: &[&str]) -> ProtocolV2Result {
        if parts.len() != 3 {
            return ServiceError::bad_request("Invalid Register message format");
        }
        let username = parts[1].to_string();
        let email = parts[2].to_string();

        self.player_service.try_register(&username, &email)?;
        Ok(None)
    }

    pub fn handle_reset_token_message(&self, parts: &[&str]) -> ProtocolV2Result {
        if parts.len() != 3 {
            return ServiceError::bad_request("Invalid SendResetToken message format");
        }
        let username = parts[1].to_string();
        let email = parts[2].to_string();

        self.player_service.send_reset_token(&username, &email)?;
        Ok(None)
    }

    pub fn handle_reset_password_message(&self, parts: &[&str]) -> ProtocolV2Result {
        if parts.len() != 4 {
            return ServiceError::bad_request("Invalid ResetPassword message format");
        }
        let username = parts[1].to_string();
        let token = parts[2].to_string();
        let new_password = parts[3].to_string();

        self.player_service
            .reset_password(&username, &token, &new_password)?;
        Ok(None)
    }

    pub fn handle_change_password_message(
        &self,
        username: &PlayerUsername,
        parts: &[&str],
    ) -> ProtocolV2Result {
        if parts.len() != 3 {
            return ServiceError::bad_request("Invalid ChangePassword message format");
        }
        let old_password = parts[1].to_string();
        let new_password = parts[2].to_string();

        self.player_service
            .change_password(username, &old_password, &new_password)?;
        Ok(None)
    }
}
