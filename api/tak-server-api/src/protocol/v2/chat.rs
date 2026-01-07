use log::error;
use tak_server_app::{
    domain::{AccountId, ListenerId, PlayerId, moderation::ModerationFlag},
    ports::notification::ChatMessageSource,
};

use crate::{
    app::ServiceError,
    client::ConnectionId,
    protocol::v2::{ProtocolV2Handler, V2Response, split_n_and_rest},
};

impl ProtocolV2Handler {
    pub async fn send_chat_message(
        &self,
        id: ConnectionId,
        from_player_id: PlayerId,
        message: &str,
        source: &ChatMessageSource,
    ) {
        let Some(username) = self
            .app
            .get_account_workflow
            .get_account(from_player_id)
            .await
            .ok()
            .map(|a| a.username)
        else {
            error!(
                "Failed to get username for player ID {} when handling chat message",
                from_player_id
            );
            return;
        };
        let msg = match source {
            ChatMessageSource::Global => format!("Shout <{}> {}", username, message),
            ChatMessageSource::Room(name) => {
                format!("ShoutRoom {} <{}> {}", name, username, message)
            }
            ChatMessageSource::Private => format!("Tell <{}> {}", username, message),
        };
        self.send_to(id, msg);
    }

    pub async fn handle_room_membership_message(
        &self,
        id: ConnectionId,
        listener_id: ListenerId,
        parts: &[&str],
        join: bool,
    ) -> V2Response {
        if parts.len() != 2 {
            return V2Response::ErrorNOK(ServiceError::BadRequest(
                "Invalid JoinRoom/LeaveRoom message format".to_string(),
            ));
        }
        let room = parts[1].to_string();
        if join {
            self.app.chat_room_use_case.join_room(&room, listener_id);
            self.send_to(id, format!("Joined room {}", room));
        } else {
            self.app.chat_room_use_case.leave_room(&room, listener_id);
            self.send_to(id, format!("Left room {}", room));
        }
        V2Response::OK
    }

    pub async fn handle_shout_message(
        &self,
        id: ConnectionId,
        account_id: AccountId,
        player_id: PlayerId,
        orig_msg: &str,
    ) -> V2Response {
        let (parts, msg) = split_n_and_rest(orig_msg, 1);
        if parts.len() != 1 || msg.is_empty() {
            return V2Response::ErrorNOK(ServiceError::BadRequest(
                "Invalid Shout message format".to_string(),
            ));
        }
        let Some(account) = self.auth.get_account(&account_id).await else {
            return V2Response::ErrorNOK(ServiceError::Internal(
                "Failed to retrieve account information".to_string(),
            ));
        };
        if account.is_flagged(ModerationFlag::Silenced) {
            let username = match self.app.get_account_workflow.get_account(player_id).await {
                Ok(account) => account.username,
                Err(_) => {
                    return V2Response::ErrorNOK(ServiceError::Internal(
                        "Failed to retrieve username".to_string(),
                    ));
                }
            };
            let msg = format!(
                "Shout <{}> {}",
                username, "<Server: You have been silenced for inappropriate chat behavior.>"
            );
            self.send_to(id, msg);
            return V2Response::OK;
        }

        self.app
            .chat_message_use_case
            .send_global_message(player_id, &msg)
            .await;
        V2Response::OK
    }

    pub async fn handle_shout_room_message(
        &self,
        id: ConnectionId,
        account_id: AccountId,
        player_id: PlayerId,
        orig_msg: &str,
    ) -> V2Response {
        let (parts, msg) = split_n_and_rest(orig_msg, 2);
        if parts.len() != 2 || msg.is_empty() {
            return V2Response::ErrorNOK(ServiceError::BadRequest(
                "Invalid ShoutRoom message format".to_string(),
            ));
        }
        let room = parts[1].to_string();

        let Some(account) = self.auth.get_account(&account_id).await else {
            return V2Response::ErrorNOK(ServiceError::Internal(
                "Failed to retrieve account information".to_string(),
            ));
        };
        if account.is_flagged(ModerationFlag::Silenced) {
            let username = match self.app.get_account_workflow.get_account(player_id).await {
                Ok(account) => account.username,
                Err(_) => {
                    return V2Response::ErrorNOK(ServiceError::Internal(
                        "Failed to retrieve username".to_string(),
                    ));
                }
            };
            let msg = format!(
                "ShoutRoom {} <{}> {}",
                room, username, "<Server: You have been silenced for inappropriate chat behavior.>"
            );
            self.send_to(id, msg);
            return V2Response::OK;
        }

        self.app
            .chat_message_use_case
            .send_room_message(player_id, &room, &msg)
            .await;
        V2Response::OK
    }

    pub async fn handle_tell_message(
        &self,
        account_id: AccountId,
        player_id: PlayerId,
        orig_msg: &str,
    ) -> V2Response {
        let (parts, msg) = split_n_and_rest(orig_msg, 2);
        if parts.len() != 2 || msg.is_empty() {
            return V2Response::ErrorNOK(ServiceError::BadRequest(
                "Invalid Tell message format".to_string(),
            ));
        }
        let target_username = parts[1];
        let Some((target_player_id, _)) = self
            .acl
            .get_account_and_player_id_by_username(target_username)
            .await
        else {
            return V2Response::ErrorNOK(ServiceError::BadRequest(format!(
                "No such user: {}",
                target_username
            )));
        };
        let Some(account) = self.auth.get_account(&account_id).await else {
            return V2Response::ErrorNOK(ServiceError::Internal(
                "Failed to retrieve account information".to_string(),
            ));
        };
        if account.is_flagged(ModerationFlag::Silenced) {
            return V2Response::Message(format!(
                "Told <{}> <Server: You have been silenced for inappropriate chat behavior.>",
                target_username
            ));
        }
        let sent_msg = self
            .app
            .chat_message_use_case
            .send_private_message(player_id, target_player_id, &msg)
            .await;
        V2Response::Message(format!("Told <{}> {}", target_username, sent_msg))
    }
}
