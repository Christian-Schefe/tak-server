use tak_server_domain::{ServiceResult, player::PlayerUsername};

use crate::protocol::json::{ClientResponse, ProtocolJsonHandler};

impl ProtocolJsonHandler {
    pub fn handle_chat_message(
        &self,
        username: &PlayerUsername,
        message: &str,
        room: &Option<String>,
        player: &Option<PlayerUsername>,
    ) -> ServiceResult<ClientResponse> {
        if let Some(room) = room {
            self.chat_service
                .send_message_to_room(username, room, message)?;
        } else if let Some(player) = player {
            self.chat_service
                .send_message_to_player(username, &player.to_string(), message)?;
        } else {
            self.chat_service.send_message_to_all(username, message)?;
        }
        Ok(ClientResponse::Ok)
    }
}
