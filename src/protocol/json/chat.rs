use tak_server_domain::{ServiceResult, player::PlayerUsername};

use crate::protocol::json::{ClientResponse, ProtocolJsonHandler};

impl ProtocolJsonHandler {
    pub async fn handle_chat_message(
        &self,
        username: &PlayerUsername,
        message: &str,
        room: &Option<String>,
        player: &Option<PlayerUsername>,
    ) -> ServiceResult<ClientResponse> {
        if let Some(room) = room {
            self.chat_service
                .send_message_to_room(username, room, message)
                .await?;
        } else if let Some(player) = player {
            self.chat_service
                .send_message_to_player(username, &player.to_string(), message)
                .await?;
        } else {
            self.chat_service
                .send_message_to_all(username, message)
                .await?;
        }
        Ok(ClientResponse::Ok)
    }
}
