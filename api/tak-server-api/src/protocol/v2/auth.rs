use tak_server_app::domain::{AccountId, ListenerId, moderation::AccountRole};

use crate::{
    app::ServiceError,
    protocol::v2::{ProtocolV2Handler, V2Response},
};

fn random_guest_token() -> String {
    uuid::Uuid::new_v4().to_string()
}

impl ProtocolV2Handler {
    pub async fn handle_login_message(&self, id: ListenerId, parts: &[&str]) -> V2Response {
        if parts.len() >= 2 && parts[1] == "Guest" {
            let token = parts
                .get(2)
                .map(|s| s.to_string())
                .unwrap_or_else(|| random_guest_token());
            let account = self.auth.get_or_create_guest_account(&token).await;
            return match self
                .transport
                .associate_account(id, account.account_id)
                .await
            {
                Ok(_) => V2Response::Message(format!("Welcome {}!", account.username)),
                Err(e) => V2Response::ErrorMessage(
                    ServiceError::BadRequest(e),
                    "Authentication failure".to_string(),
                ),
            };
        }
        if parts.len() != 3 {
            return V2Response::ErrorNOK(ServiceError::BadRequest(
                "Invalid Login message format".to_string(),
            ));
        }
        let username = parts[1].to_string();
        let password = parts[2].to_string();

        let account = match self.acl.login_username_password(&username, &password).await {
            Ok(acc) => acc,
            Err(e) => {
                return V2Response::ErrorMessage(
                    ServiceError::Unauthorized(e),
                    "Authentication failure".to_string(),
                );
            }
        };

        match self
            .transport
            .associate_account(id, account.account_id)
            .await
        {
            Ok(_) => {}
            Err(e) => {
                return V2Response::ErrorMessage(
                    ServiceError::BadRequest(e),
                    "Authentication failure".to_string(),
                );
            }
        };

        if matches!(account.role, AccountRole::Moderator | AccountRole::Admin) {
            self.send_to(id, "Is Mod");
        }

        V2Response::Message(format!("Welcome {}!", username))
    }

    pub async fn handle_register_message(&self, parts: &[&str]) -> V2Response {
        if parts.len() != 3 {
            return V2Response::ErrorNOK(ServiceError::BadRequest(
                "Invalid Register message format".to_string(),
            ));
        }
        let username = parts[1].to_string();
        let email = parts[2].to_string();

        match self.acl.register_username_email(&username, &email).await {
            Err(e) => {
                return V2Response::ErrorMessage(
                    ServiceError::BadRequest(e.clone()),
                    format!("Registration Error: {}", e),
                );
            }
            Ok(_) => {}
        }

        V2Response::Message(format!(
            "Registered {}. Check your email for the password.",
            username
        ))
    }

    pub async fn handle_reset_token_message(&self, parts: &[&str]) -> V2Response {
        if parts.len() != 3 {
            return V2Response::ErrorNOK(ServiceError::BadRequest(
                "Invalid ResetToken message format".to_string(),
            ));
        }
        let username = parts[1].to_string();
        let email = parts[2].to_string();

        if let Err(e) = self.acl.request_password_reset(&username, &email).await {
            return V2Response::ErrorMessage(
                ServiceError::BadRequest(e.clone()),
                format!("Reset Token Error: {}", e),
            );
        }
        V2Response::Message("Reset token sent".to_string())
    }

    pub async fn handle_reset_password_message(&self, parts: &[&str]) -> V2Response {
        if parts.len() != 4 {
            return V2Response::ErrorNOK(ServiceError::BadRequest(
                "Invalid ResetPassword message format".to_string(),
            ));
        }
        let username = parts[1].to_string();
        let token = parts[2].to_string();
        let new_password = parts[3].to_string();

        match self
            .acl
            .reset_password(&username, &token, &new_password)
            .await
        {
            Ok(_) => V2Response::Message("Password is changed".to_string()),
            Err(e) => {
                V2Response::ErrorMessage(ServiceError::Unauthorized(e), "Wrong token".to_string())
            }
        }
    }

    pub async fn handle_change_password_message(
        &self,
        account_id: AccountId,
        parts: &[&str],
    ) -> V2Response {
        if parts.len() != 3 {
            return V2Response::ErrorNOK(ServiceError::BadRequest(
                "Invalid ChangePassword message format".to_string(),
            ));
        }
        let old_password = parts[1].to_string();
        let new_password = parts[2].to_string();

        let username = match self.auth.get_account(&account_id).await {
            Some(acc) => acc.username,
            None => {
                return V2Response::ErrorMessage(
                    ServiceError::BadRequest("Account not found".to_string()),
                    "Account not found".to_string(),
                );
            }
        };

        match self
            .acl
            .change_password(&username, &old_password, &new_password)
            .await
        {
            Ok(_) => V2Response::Message("Password changed".to_string()),
            Err(e) => V2Response::ErrorMessage(
                ServiceError::Unauthorized(e),
                "Wrong password".to_string(),
            ),
        }
    }
}
