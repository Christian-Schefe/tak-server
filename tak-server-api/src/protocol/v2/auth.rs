use tak_server_domain::{ServiceError, player::PlayerUsername, transport::ListenerId};

use crate::protocol::v2::{ProtocolV2Handler, ProtocolV2Result};

impl ProtocolV2Handler {
    pub async fn handle_login_message(&self, id: ListenerId, parts: &[&str]) -> ProtocolV2Result {
        if parts.len() >= 2 && parts[1] == "Guest" {
            let token = parts.get(2).copied();
            let username = self.app_state.player_service.try_login_guest(token)?;
            self.transport.associate_player(id, &username).await?;
            return Ok(Some(format!("Welcome {}!", username)));
        }
        if parts.len() != 3 {
            return ServiceError::bad_request("Invalid Login message format");
        }
        let username = parts[1].to_string();
        let password = parts[2].to_string();

        if let Err(e) = self
            .app_state
            .player_service
            .try_login(&username, &password)
            .await
        {
            let _ = self.send_to(id, format!("Authentication failure: {}", e));
            return Err(e);
        }
        self.transport.associate_player(id, &username).await?;
        Ok(Some(format!("Welcome {}!", username)))
    }

    pub async fn handle_login_token_message(
        &self,
        id: ListenerId,
        parts: &[&str],
    ) -> ProtocolV2Result {
        if parts.len() != 2 {
            return ServiceError::bad_request("Invalid LoginToken message format");
        }
        let token = parts[1];
        let username = match self.app_state.player_service.try_login_jwt(token).await {
            Ok(name) => name,
            Err(e) => {
                let _ = self.send_to(id, format!("Authentication failure: {}", e));
                return Err(e);
            }
        };
        self.transport.associate_player(id, &username).await?;
        Ok(Some(format!("Welcome {}!", username)))
    }

    pub async fn handle_register_message(
        &self,
        id: ListenerId,
        parts: &[&str],
    ) -> ProtocolV2Result {
        if parts.len() != 3 {
            return ServiceError::bad_request("Invalid Register message format");
        }
        let username = parts[1].to_string();
        let email = parts[2].to_string();

        if let Err(e) = self
            .app_state
            .player_service
            .try_register(&username, &email)
            .await
        {
            let _ = self.send_to(id, format!("Registration Error: {}", e));
            return Err(e);
        }

        Ok(Some(format!(
            "Registered {}. Check your email for the password.",
            username
        )))
    }

    pub async fn handle_reset_token_message(
        &self,
        id: ListenerId,
        parts: &[&str],
    ) -> ProtocolV2Result {
        if parts.len() != 3 {
            return ServiceError::bad_request("Invalid SendResetToken message format");
        }
        let username = parts[1].to_string();
        let email = parts[2].to_string();

        if let Err(e) = self
            .app_state
            .player_service
            .send_reset_token(&username, &email)
            .await
        {
            let _ = self.send_to(id, format!("Reset Token Error: {}", e));
            return Err(e);
        }
        Ok(None)
    }

    pub async fn handle_reset_password_message(&self, parts: &[&str]) -> ProtocolV2Result {
        if parts.len() != 4 {
            return ServiceError::bad_request("Invalid ResetPassword message format");
        }
        let username = parts[1].to_string();
        let token = parts[2].to_string();
        let new_password = parts[3].to_string();

        self.app_state
            .player_service
            .reset_password(&username, &token, &new_password)
            .await?;
        Ok(None)
    }

    pub async fn handle_change_password_message(
        &self,
        id: ListenerId,
        username: &PlayerUsername,
        parts: &[&str],
    ) -> ProtocolV2Result {
        if parts.len() != 3 {
            return ServiceError::bad_request("Invalid ChangePassword message format");
        }
        let old_password = parts[1].to_string();
        let new_password = parts[2].to_string();

        match self
            .app_state
            .player_service
            .change_password(username, &old_password, &new_password)
            .await
        {
            Ok(_) => Ok(Some("Password changed".to_string())),
            Err(e) => {
                let _ = self.send_to(id, format!("Error: {}", e));
                Err(e)
            }
        }
    }
}
