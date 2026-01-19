use log::error;
use tak_player_connection::ConnectionId;
use tak_server_app::{
    domain::{AccountId, moderation::ModerationFlag},
    workflow::chat::message::MessageTarget,
};

use crate::{
    app::ServiceError,
    protocol::v2::{ProtocolV2Handler, V2Response, split_n_and_rest},
};

impl ProtocolV2Handler {
    pub async fn send_chat_message(
        &self,
        id: ConnectionId,
        this_account_id: Option<&AccountId>,
        from_account_id: &AccountId,
        message: &str,
        target: &MessageTarget,
    ) {
        let Some(account) = self.auth.get_account(from_account_id).await else {
            error!("Failed to retrieve account information for sending chat message");
            return;
        };
        let msg = match target {
            MessageTarget::Global => format!("Shout <{}> {}", account.username, message),
            MessageTarget::Room(name) => {
                format!("ShoutRoom {} <{}> {}", name, account.username, message)
            }
            MessageTarget::Private(to_account_id) => {
                if this_account_id.is_some_and(|x| x != to_account_id) {
                    format!("Told <{}> {}", account.username, message)
                } else {
                    format!("Tell <{}> {}", account.username, message)
                }
            }
        };
        self.send_to(id, msg);
    }

    pub async fn handle_room_membership_message(
        &self,
        id: ConnectionId,
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
            self.app.chat_room_use_case.join_room(&room, id.0);
            self.send_to(id, format!("Joined room {}", room));
        } else {
            self.app.chat_room_use_case.leave_room(&room, id.0);
            self.send_to(id, format!("Left room {}", room));
        }
        V2Response::OK
    }

    pub async fn handle_shout_message(
        &self,
        id: ConnectionId,
        account_id: &AccountId,
        orig_msg: &str,
    ) -> V2Response {
        let (parts, msg) = split_n_and_rest(orig_msg, 1);
        if parts.len() != 1 || msg.is_empty() {
            return V2Response::ErrorNOK(ServiceError::BadRequest(
                "Invalid Shout message format".to_string(),
            ));
        }
        let Some(account) = self.auth.get_account(account_id).await else {
            return V2Response::ErrorNOK(ServiceError::Internal(
                "Failed to retrieve account information".to_string(),
            ));
        };
        if account.is_flagged(ModerationFlag::Silenced) {
            let msg = format!(
                "Shout <{}> {}",
                account.username,
                "<Server: You have been silenced for inappropriate chat behavior.>"
            );
            self.send_to(id, msg);
            return V2Response::OK;
        }

        self.app
            .chat_message_use_case
            .send_message(account_id, MessageTarget::Global, &msg)
            .await;
        V2Response::OK
    }

    pub async fn handle_shout_room_message(
        &self,
        id: ConnectionId,
        account_id: &AccountId,
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
            let msg = format!(
                "ShoutRoom {} <{}> {}",
                room,
                account.username,
                "<Server: You have been silenced for inappropriate chat behavior.>"
            );
            self.send_to(id, msg);
            return V2Response::OK;
        }

        self.app
            .chat_message_use_case
            .send_message(account_id, MessageTarget::Room(room), &msg)
            .await;
        V2Response::OK
    }

    pub async fn handle_tell_message(&self, account_id: &AccountId, orig_msg: &str) -> V2Response {
        let (parts, msg) = split_n_and_rest(orig_msg, 2);
        if parts.len() != 2 || msg.is_empty() {
            return V2Response::ErrorNOK(ServiceError::BadRequest(
                "Invalid Tell message format".to_string(),
            ));
        }
        let target_username = parts[1];
        let Some(target_account) = self.acl.get_account_by_username(target_username).await else {
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
        self.app
            .chat_message_use_case
            .send_message(
                account_id,
                MessageTarget::Private(target_account.account_id),
                &msg,
            )
            .await;
        V2Response::OK
    }
}
