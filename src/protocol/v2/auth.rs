use crate::{client::ClientId, player::PlayerUsername, protocol::v2::ProtocolV2Handler};

impl ProtocolV2Handler {
    pub fn handle_login_message(&self, id: &ClientId, parts: &[&str]) {
        if parts.len() >= 2 && parts[1] == "Guest" {
            let token = parts.get(2).copied();
            match self.player_service.try_login_guest(id, token) {
                Ok(username) => {
                    self.send_to(id, format!("Welcome {}!", username));
                }
                Err(e) => {
                    println!("Guest login failed for user {}: {}", id, e);
                    self.send_to(id, "NOK");
                }
            }
            return;
        }
        if parts.len() != 3 {
            self.send_to(id, "NOK");
        }
        let username = parts[1].to_string();
        let password = parts[2].to_string();

        if let Err(e) = self.player_service.try_login(id, &username, &password) {
            println!("Login failed for user {}: {}", id, e);
            self.send_to(id, "NOK");
        } else {
            self.send_to(id, format!("Welcome {}!", username));
        }
    }

    pub fn handle_login_token_message(&self, id: &ClientId, parts: &[&str]) {
        if parts.len() != 2 {
            self.send_to(id, "NOK");
            return;
        }
        let token = parts[1];
        match self.player_service.try_login_jwt(id, token) {
            Ok(username) => {
                self.send_to(id, format!("Welcome {}!", username));
            }
            Err(e) => {
                println!("Login with token failed for user {}: {}", id, e);
                self.send_to(id, "NOK");
            }
        }
    }

    pub fn handle_register_message(&self, id: &ClientId, parts: &[&str]) {
        if parts.len() != 3 {
            self.send_to(id, "NOK");
            return;
        }
        let username = parts[1].to_string();
        let email = parts[2].to_string();

        if let Err(e) = self.player_service.try_register(&username, &email) {
            println!("Error registering user {}: {}", username, e);
            self.send_to(id, format!("Registration Error: {}", e));
        } else {
            self.send_to(
                id,
                format!(
                    "Registered {}. Check your email for the temporary password",
                    username
                ),
            );
        }
    }

    pub fn handle_reset_token_message(&self, id: &ClientId, parts: &[&str]) {
        if parts.len() != 3 {
            self.send_to(id, "NOK");
            return;
        }
        let username = parts[1].to_string();
        let email = parts[2].to_string();

        if let Err(e) = self.player_service.send_reset_token(&username, &email) {
            println!("Error sending reset token to user {}: {}", username, e);
            self.send_to(id, format!("Send Reset Token Error: {}", e));
        } else {
            self.send_to(id, "Reset token sent. Check your email for the token.");
        }
    }

    pub fn handle_reset_password_message(&self, id: &ClientId, parts: &[&str]) {
        if parts.len() != 4 {
            self.send_to(id, "NOK");
            return;
        }
        let username = parts[1].to_string();
        let token = parts[2].to_string();
        let new_password = parts[3].to_string();

        if let Err(e) = self
            .player_service
            .reset_password(&username, &token, &new_password)
        {
            println!("Error resetting password for client {}: {}", id, e);
            self.send_to(id, format!("Password Reset Error: {}", e));
        } else {
            self.send_to(
                id,
                "Password reset. Check your email for the temporary password.",
            );
        }
    }

    pub fn handle_change_password_message(
        &self,
        id: &ClientId,
        username: &PlayerUsername,
        parts: &[&str],
    ) {
        if parts.len() != 3 {
            self.send_to(id, "NOK");
            return;
        }
        let old_password = parts[1].to_string();
        let new_password = parts[2].to_string();

        if let Err(e) = self
            .player_service
            .change_password(username, &old_password, &new_password)
        {
            println!("Error changing password for client {}: {}", id, e);
            self.send_to(id, format!("Change Password Error: {}", e));
        } else {
            self.send_to(id, "Password changed successfully.");
        }
    }
}
