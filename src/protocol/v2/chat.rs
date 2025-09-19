use rustrict::CensorStr;

use crate::{
    client::ClientId,
    player::PlayerUsername,
    protocol::{ChatMessageSource, ServerMessage, v2::ProtocolV2Handler},
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
                    ChatMessageSource::Room(room) => {
                        format!("ShoutRoom {} <{}> {}", room, from, message)
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

    pub fn handle_room_membership_message(&self, id: &ClientId, parts: &[&str], join: bool) {
        if parts.len() != 2 {
            self.send_to(id, "NOK");
            return;
        }
        let room = parts[1];
        if join {
            self.chat_service.join_room(id, room);
        } else {
            self.chat_service.leave_room(id, room);
        }
    }

    pub fn handle_shout_message(&self, id: &ClientId, username: &PlayerUsername, msg: &str) {
        let msg = msg.replacen("Shout ", "", 1);
        if msg.is_empty() {
            self.send_to(id, "NOK");
            return;
        }
        if let Err(e) = self.chat_service.send_message_to_all(username, &msg) {
            println!("Error handling Shout message: {}", e);
            self.send_to(id, "NOK");
        }
    }

    pub fn handle_shout_room_message(
        &self,
        id: &ClientId,
        username: &PlayerUsername,
        parts: &[&str],
        msg: &str,
    ) {
        if parts.len() < 3 {
            self.send_to(id, "NOK");
            return;
        }
        let room = parts[1];
        let msg = msg.replacen(&format!("ShoutRoom {} ", room), "", 1);
        if msg.is_empty() {
            self.send_to(id, "NOK");
            return;
        }
        if let Err(e) = self.chat_service.send_message_to_room(username, room, &msg) {
            println!("Error handling Shout message: {}", e);
            self.send_to(id, "NOK");
        }
    }

    pub fn handle_tell_message(
        &self,
        id: &ClientId,
        username: &PlayerUsername,
        parts: &[&str],
        msg: &str,
    ) {
        if parts.len() < 3 {
            self.send_to(id, "NOK");
            return;
        }
        let target_username = parts[1];
        let msg = msg.replacen(&format!("Tell {} ", target_username), "", 1);

        self.send_to(id, format!("Told <{}> {}", target_username, msg.censor()));
        if msg.is_empty() {
            self.send_to(id, "NOK");
            return;
        }
        if let Err(e) =
            self.chat_service
                .send_message_to_player(username, &target_username.to_string(), &msg)
        {
            println!("Error handling Tell message: {}", e);
            self.send_to(id, "NOK");
        }
    }
}
