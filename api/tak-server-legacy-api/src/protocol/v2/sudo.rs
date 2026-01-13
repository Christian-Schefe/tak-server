use tak_player_connection::ConnectionId;
use tak_server_app::{
    domain::AccountId,
    ports::notification::{ListenerMessage, ServerAlertMessage},
    workflow::account::moderate::ModerationError,
};

use crate::{
    app::ServiceError,
    client::DisconnectReason,
    protocol::v2::{ProtocolV2Handler, V2Response, split_n_and_rest},
};

impl ProtocolV2Handler {
    pub async fn handle_sudo_message(
        &self,
        id: ConnectionId,
        account_id: &AccountId,
        msg: &str,
        parts: &[&str],
    ) -> V2Response {
        if parts.len() < 2 {
            return V2Response::ErrorNOK(ServiceError::BadRequest(
                "Invalid Sudo command format".to_string(),
            ));
        }
        let command = parts[1];
        self.send_to(id, format!("sudoReply > {}", msg));
        let response = match command {
            "ban" => self.handle_ban_message(account_id, msg, true).await,
            "unban" => self.handle_ban_message(account_id, msg, false).await,
            "gag" => {
                self.handle_player_update(account_id, parts, Some(true), None, None, None)
                    .await
            }
            "ungag" => {
                self.handle_player_update(account_id, parts, Some(false), None, None, None)
                    .await
            }
            "mod" => {
                self.handle_player_update(account_id, parts, None, Some(true), None, None)
                    .await
            }
            "unmod" => {
                self.handle_player_update(account_id, parts, None, Some(false), None, None)
                    .await
            }
            "admin" => {
                self.handle_player_update(account_id, parts, None, None, Some(true), None)
                    .await
            }
            "unadmin" => {
                self.handle_player_update(account_id, parts, None, None, Some(false), None)
                    .await
            }
            "bot" => {
                self.handle_player_update(account_id, parts, None, None, None, Some(true))
                    .await
            }
            "unbot" => {
                self.handle_player_update(account_id, parts, None, None, None, Some(false))
                    .await
            }
            "kick" => self.handle_kick_message(account_id, parts).await,
            "list" => V2Response::OK,   // Not supported anymore
            "reload" => V2Response::OK, // Was used in legacy profanity filter, no-op here.
            "broadcast" => self.handle_broadcast_message(parts, msg).await,
            "set" => V2Response::OK, // Not supported anymore
            _ => V2Response::ErrorNOK(ServiceError::BadRequest("Unknown Sudo command".to_string())),
        };
        match response {
            V2Response::Message(msg) => V2Response::Message(format!("sudoReply {}", msg)),
            V2Response::ErrorMessage(e, msg) => {
                V2Response::ErrorMessage(e, format!("sudoReply {}", msg))
            }
            V2Response::OK => V2Response::Message("sudoReply Success".to_string()),
            V2Response::ErrorNOK(e) => {
                let err_str = format!("sudoReply {}", e);
                V2Response::ErrorMessage(e, err_str)
            }
        }
    }

    async fn handle_player_update(
        &self,
        account_id: &AccountId,
        parts: &[&str],
        silenced: Option<bool>,
        modded: Option<bool>,
        admin: Option<bool>,
        bot: Option<bool>,
    ) -> V2Response {
        if parts.len() != 3 {
            return V2Response::ErrorNOK(ServiceError::BadRequest(
                "Invalid Sudo command format".to_string(),
            ));
        }
        let target_username = parts[2].to_string();
        let Some(target_account) = self.acl.get_account_by_username(&target_username).await else {
            return V2Response::ErrorNOK(ServiceError::BadRequest(format!(
                "No such user: {}",
                target_username
            )));
        };

        if let Some(silenced) = silenced {
            let res = if silenced {
                self.app
                    .account_moderate_use_case
                    .silence_player(account_id, &target_account.account_id)
                    .await
            } else {
                self.app
                    .account_moderate_use_case
                    .unsilence_player(account_id, &target_account.account_id)
                    .await
            };
            match res {
                Ok(()) => {}
                Err(ModerationError::AccountNotFound) => {
                    return V2Response::ErrorNOK(ServiceError::BadRequest(format!(
                        "No account found for user: {}",
                        target_username
                    )));
                }
                Err(ModerationError::InsufficientPermissions) => {
                    return V2Response::ErrorNOK(ServiceError::BadRequest(
                        "Insufficient permissions to silence user".to_string(),
                    ));
                }
            }

            V2Response::Message(format!(
                "{} {}",
                target_username,
                if silenced { "gagged" } else { "ungagged" }
            ))
        } else if let Some(modded) = modded {
            let res = if modded {
                self.app
                    .account_moderate_use_case
                    .set_moderator(account_id, &target_account.account_id)
                    .await
            } else {
                self.app
                    .account_moderate_use_case
                    .set_user(account_id, &target_account.account_id)
                    .await
            };

            match res {
                Ok(()) => {}

                Err(ModerationError::AccountNotFound) => {
                    return V2Response::ErrorNOK(ServiceError::BadRequest(format!(
                        "No account found for user: {}",
                        target_username
                    )));
                }
                Err(ModerationError::InsufficientPermissions) => {
                    return V2Response::ErrorNOK(ServiceError::BadRequest(
                        "Insufficient permissions to modify moderator status".to_string(),
                    ));
                }
            }

            V2Response::Message(format!(
                "{} {} as moderator",
                if modded { "Added" } else { "Removed" },
                target_username
            ))
        } else if let Some(admin) = admin {
            let res = if admin {
                self.app
                    .account_moderate_use_case
                    .set_admin(account_id, &target_account.account_id)
                    .await
            } else {
                self.app
                    .account_moderate_use_case
                    .set_user(account_id, &target_account.account_id)
                    .await
            };

            match res {
                Ok(()) => {}
                Err(ModerationError::AccountNotFound) => {
                    return V2Response::ErrorNOK(ServiceError::BadRequest(format!(
                        "No account found for user: {}",
                        target_username
                    )));
                }
                Err(ModerationError::InsufficientPermissions) => {
                    return V2Response::ErrorNOK(ServiceError::BadRequest(
                        "Insufficient permissions to modify admin status".to_string(),
                    ));
                }
            }

            V2Response::Message(format!(
                "{} {} as admin",
                if admin { "Added" } else { "Removed" },
                target_username
            ))
        } else if let Some(_) = bot {
            V2Response::ErrorNOK(ServiceError::NotPossible("No longer supported".to_string()))
        } else {
            V2Response::ErrorNOK(ServiceError::BadRequest(
                "No valid player update specified".to_string(),
            ))
        }
    }

    async fn handle_kick_message(&self, account_id: &AccountId, parts: &[&str]) -> V2Response {
        if parts.len() != 3 {
            return V2Response::ErrorNOK(ServiceError::BadRequest(
                "Invalid Sudo kick command format".to_string(),
            ));
        }
        let target_username = parts[2].to_string();
        let target_account = match self.acl.get_account_by_username(&target_username).await {
            Some(account) => account,
            None => {
                return V2Response::ErrorNOK(ServiceError::BadRequest(format!(
                    "No such user: {}",
                    target_username
                )));
            }
        };

        match self
            .app
            .account_moderate_use_case
            .kick_player(account_id, &target_account.account_id)
            .await
        {
            Ok(()) => {}
            Err(ModerationError::AccountNotFound) => {
                return V2Response::ErrorNOK(ServiceError::BadRequest(format!(
                    "No account found for user: {}",
                    target_username
                )));
            }
            Err(ModerationError::InsufficientPermissions) => {
                return V2Response::ErrorNOK(ServiceError::BadRequest(
                    "Insufficient permissions to kick user".to_string(),
                ));
            }
        }

        self.transport
            .close_account_with_reason(&target_account.account_id, DisconnectReason::Kick)
            .await;
        V2Response::Message(format!("{} kicked", target_username))
    }

    async fn handle_ban_message(
        &self,
        account_id: &AccountId,
        orig_msg: &str,
        ban: bool,
    ) -> V2Response {
        let (parts, msg) = split_n_and_rest(orig_msg, 3);
        if parts.len() != 3 {
            return V2Response::ErrorNOK(ServiceError::BadRequest(
                "Invalid Ban/Unban message format".to_string(),
            ));
        }
        let target_username = parts[2].to_string();
        let Some(target_account) = self.acl.get_account_by_username(&target_username).await else {
            return V2Response::ErrorNOK(ServiceError::BadRequest(format!(
                "No such user: {}",
                target_username
            )));
        };

        let res = if ban {
            self.app
                .account_moderate_use_case
                .ban_player(account_id, &target_account.account_id, msg)
                .await
        } else {
            self.app
                .account_moderate_use_case
                .unban_player(account_id, &target_account.account_id)
                .await
        };
        match res {
            Ok(()) => {}
            Err(ModerationError::AccountNotFound) => {
                return V2Response::ErrorNOK(ServiceError::BadRequest(format!(
                    "No account found for user: {}",
                    target_username
                )));
            }
            Err(ModerationError::InsufficientPermissions) => {
                return V2Response::ErrorNOK(ServiceError::BadRequest(
                    "Insufficient permissions to ban/unban user".to_string(),
                ));
            }
        }

        V2Response::Message(format!(
            "{} {}",
            target_username,
            if ban { "banned" } else { "unbanned" }
        ))
    }

    async fn handle_broadcast_message(&self, parts: &[&str], orig_msg: &str) -> V2Response {
        if parts.len() < 3 {
            return V2Response::ErrorNOK(ServiceError::BadRequest(
                "Invalid Sudo broadcast command format".to_string(),
            ));
        }
        let message = orig_msg
            .strip_prefix("sudo broadcast ")
            .unwrap_or("")
            .to_string();
        self.transport.notify_all(ListenerMessage::ServerAlert {
            message: ServerAlertMessage::Custom(message),
        });
        V2Response::Message("Broadcast sent".to_string())
    }
}
