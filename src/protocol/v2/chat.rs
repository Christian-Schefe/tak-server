use tak_server_domain::{ServiceError, player::PlayerUsername, transport::ChatMessageSource};

use crate::{
    client::ClientId,
    protocol::{
        ServerMessage,
        v2::{ProtocolV2Handler, ProtocolV2Result, split_n_and_rest},
    },
};

impl ProtocolV2Handler {
    pub fn handle_server_chat_message(&self, id: &ClientId, msg: &ServerMessage) {
        match msg {
            ServerMessage::ChatMessage {
                from,
                message,
                source,
            } => {
                let msg = match source {
                    ChatMessageSource::Global => format!("Shout <{}> {}", from, message),
                    ChatMessageSource::Room { name } => {
                        format!("ShoutRoom {} <{}> {}", name, from, message)
                    }
                    ChatMessageSource::Private => format!("Tell <{}> {}", from, message),
                };
                self.send_to(id, msg);
            }
            ServerMessage::RoomMembership { room, joined } => {
                let msg = if *joined {
                    format!("Joined room {}", room)
                } else {
                    format!("Left room {}", room)
                };
                self.send_to(id, msg);
            }
            _ => {
                eprintln!("Unhandled server chat message: {:?}", msg);
            }
        }
    }

    pub fn handle_room_membership_message(
        &self,
        id: &ClientId,
        parts: &[&str],
        join: bool,
    ) -> ProtocolV2Result {
        if parts.len() != 2 {
            return ServiceError::bad_request("Invalid JoinRoom/LeaveRoom message format");
        }
        let room = parts[1].to_string();
        if join {
            self.app_state.chat_service.join_room(id, &room)?;
        } else {
            self.app_state.chat_service.leave_room(id, &room)?;
        }
        Ok(None)
    }

    pub fn handle_shout_message(
        &self,
        username: &PlayerUsername,
        orig_msg: &str,
    ) -> ProtocolV2Result {
        let (parts, msg) = split_n_and_rest(orig_msg, 1);
        if parts.len() != 1 || msg.is_empty() {
            return ServiceError::bad_request("Invalid Shout message format");
        }
        self.app_state
            .chat_service
            .send_message_to_all(username, &msg)?;
        Ok(None)
    }

    pub fn handle_shout_room_message(
        &self,
        username: &PlayerUsername,
        orig_msg: &str,
    ) -> ProtocolV2Result {
        let (parts, msg) = split_n_and_rest(orig_msg, 2);
        if parts.len() != 2 || msg.is_empty() {
            return ServiceError::bad_request("Invalid ShoutRoom message format");
        }
        let room = parts[1].to_string();

        self.app_state
            .chat_service
            .send_message_to_room(username, &room, &msg)?;
        Ok(None)
    }

    pub fn handle_tell_message(
        &self,
        username: &PlayerUsername,
        orig_msg: &str,
    ) -> ProtocolV2Result {
        let (parts, msg) = split_n_and_rest(orig_msg, 2);
        if parts.len() != 2 || msg.is_empty() {
            return ServiceError::bad_request("Invalid Tell message format");
        }
        let target_username = parts[1];

        let sent_msg = self.app_state.chat_service.send_message_to_player(
            username,
            &target_username.to_string(),
            &msg,
        )?;
        Ok(Some(format!("Told <{}> {}", target_username, sent_msg)))
    }
}
