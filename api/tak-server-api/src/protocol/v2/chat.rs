use log::error;
use tak_server_domain::{
    ServiceError,
    player::PlayerUsername,
    transport::{ChatMessageSource, ListenerId, do_player_send},
};

use crate::protocol::{
    ServerMessage,
    v2::{ProtocolV2Handler, ProtocolV2Result, V2Response, split_n_and_rest},
};

impl ProtocolV2Handler {
    pub fn handle_server_chat_message(&self, id: ListenerId, msg: &ServerMessage) {
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
                error!("Unhandled server chat message: {:?}", msg);
            }
        }
    }

    pub async fn handle_room_membership_message(
        &self,
        id: ListenerId,
        parts: &[&str],
        join: bool,
    ) -> ProtocolV2Result {
        if parts.len() != 2 {
            return ServiceError::bad_request("Invalid JoinRoom/LeaveRoom message format");
        }
        let room = parts[1].to_string();
        if join {
            self.app_state.chat_service.join_room(id, &room).await?;
        } else {
            self.app_state.chat_service.leave_room(id, &room).await?;
        }
        Ok(V2Response::OK)
    }

    pub async fn handle_shout_message(
        &self,
        username: &PlayerUsername,
        orig_msg: &str,
    ) -> ProtocolV2Result {
        let (parts, msg) = split_n_and_rest(orig_msg, 1);
        if parts.len() != 1 || msg.is_empty() {
            return ServiceError::bad_request("Invalid Shout message format");
        }
        match self
            .app_state
            .chat_service
            .send_message_to_all(username, &msg)
            .await
        {
            Ok(_) => Ok(V2Response::OK),
            Err(ServiceError::Forbidden(_)) => {
                do_player_send(
                    &self.app_state.player_connection_service,
                    &self.app_state.transport_service,
                    username,
                    &ServerMessage::ChatMessage {
                        from: username.clone(),
                        message:
                            "<Server: You have been silenced for inappropriate chat behavior.>"
                                .to_string(),
                        source: ChatMessageSource::Global,
                    },
                )
                .await;
                Ok(V2Response::OK)
            }
            Err(e) => Err(e),
        }
    }

    pub async fn handle_shout_room_message(
        &self,
        username: &PlayerUsername,
        orig_msg: &str,
    ) -> ProtocolV2Result {
        let (parts, msg) = split_n_and_rest(orig_msg, 2);
        if parts.len() != 2 || msg.is_empty() {
            return ServiceError::bad_request("Invalid ShoutRoom message format");
        }
        let room = parts[1].to_string();

        match self
            .app_state
            .chat_service
            .send_message_to_room(username, &room, &msg)
            .await
        {
            Ok(_) => Ok(V2Response::OK),
            Err(ServiceError::Forbidden(_)) => {
                do_player_send(
                    &self.app_state.player_connection_service,
                    &self.app_state.transport_service,
                    username,
                    &ServerMessage::ChatMessage {
                        from: username.clone(),
                        message:
                            "<Server: You have been silenced for inappropriate chat behavior.>"
                                .to_string(),
                        source: ChatMessageSource::Room { name: room },
                    },
                )
                .await;
                Ok(V2Response::OK)
            }
            Err(e) => Err(e),
        }
    }

    pub async fn handle_tell_message(
        &self,
        username: &PlayerUsername,
        orig_msg: &str,
    ) -> ProtocolV2Result {
        let (parts, msg) = split_n_and_rest(orig_msg, 2);
        if parts.len() != 2 || msg.is_empty() {
            return ServiceError::bad_request("Invalid Tell message format");
        }
        let target_username = parts[1];

        match self
            .app_state
            .chat_service
            .send_message_to_player(username, &target_username.to_string(), &msg)
            .await
        {
            Ok(sent_msg) => Ok(V2Response::Message(format!(
                "Told <{}> {}",
                target_username, sent_msg
            ))),
            Err(ServiceError::Forbidden(_)) => Ok(V2Response::Message(format!(
                "Told <{}> <Server: You have been silenced for inappropriate chat behavior.>",
                target_username
            ))),
            Err(e) => Err(e),
        }
    }
}
