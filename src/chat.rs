use std::sync::Arc;

use dashmap::DashMap;
use rustrict::CensorStr;

use crate::{
    client::{ClientId, ClientService},
    player::{PlayerService, PlayerUsername},
    protocol::{ChatMessageSource, ServerMessage},
};

pub trait ChatService {
    fn join_room(&self, client_id: &ClientId, room_name: &str);
    fn leave_room(&self, client_id: &ClientId, room_name: &str);
    fn send_message_to_all(&self, username: &PlayerUsername, message: &str) -> Result<(), String>;
    fn send_message_to_room(
        &self,
        username: &PlayerUsername,
        room_name: &str,
        message: &str,
    ) -> Result<(), String>;
    fn send_message_to_player(
        &self,
        from_username: &PlayerUsername,
        to_username: &PlayerUsername,
        message: &str,
    ) -> Result<(), String>;
}

pub struct ChatServiceImpl {
    client_service: Arc<Box<dyn ClientService + Send + Sync>>,
    player_service: Arc<Box<dyn PlayerService + Send + Sync>>,
    chat_rooms: Arc<DashMap<String, Vec<ClientId>>>,
}

impl ChatServiceImpl {
    pub fn new(
        client_service: Arc<Box<dyn ClientService + Send + Sync>>,
        player_service: Arc<Box<dyn PlayerService + Send + Sync>>,
    ) -> Self {
        Self {
            client_service,
            player_service,
            chat_rooms: Arc::new(DashMap::new()),
        }
    }
}

impl ChatService for ChatServiceImpl {
    fn join_room(&self, client_id: &ClientId, room_name: &str) {
        let mut room = self.chat_rooms.entry(room_name.to_string()).or_default();
        if !room.contains(client_id) {
            room.push(*client_id);
        }
        let msg = ServerMessage::RoomMembership {
            room: room_name.to_string(),
            joined: true,
        };
        self.client_service.try_protocol_send(client_id, &msg);
    }

    fn leave_room(&self, client_id: &ClientId, room_name: &str) {
        if let Some(mut room) = self.chat_rooms.get_mut(room_name) {
            room.retain(|id| id != client_id);
            if room.is_empty() {
                self.chat_rooms.remove(room_name);
            }
        }
        let msg = ServerMessage::RoomMembership {
            room: room_name.to_string(),
            joined: false,
        };
        self.client_service.try_protocol_send(client_id, &msg);
    }

    fn send_message_to_all(&self, username: &PlayerUsername, message: &str) -> Result<(), String> {
        let Some(player) = self.player_service.fetch_player(username) else {
            return Err("Associated player not found".to_string());
        };
        if player.is_gagged {
            return Err("You are gagged and cannot send messages".to_string());
        }
        let msg = ServerMessage::ChatMessage {
            from: username.clone(),
            message: message.censor(),
            source: ChatMessageSource::Global,
        };
        self.client_service.try_auth_protocol_broadcast(&msg);
        Ok(())
    }

    fn send_message_to_room(
        &self,
        username: &PlayerUsername,
        room_name: &str,
        message: &str,
    ) -> Result<(), String> {
        let Some(player) = self.player_service.fetch_player(&username) else {
            return Err("Associated player not found".to_string());
        };
        if player.is_gagged {
            return Err("You are gagged and cannot send messages".to_string());
        }
        if let Some(room) = self.chat_rooms.get(room_name) {
            let msg = ServerMessage::ChatMessage {
                from: username.clone(),
                message: message.censor(),
                source: ChatMessageSource::Room(room_name.to_string()),
            };
            self.client_service.try_protocol_multicast(&room, &msg);
        }
        Ok(())
    }

    fn send_message_to_player(
        &self,
        from_username: &PlayerUsername,
        to_username: &PlayerUsername,
        message: &str,
    ) -> Result<(), String> {
        let Some(from_player) = self.player_service.fetch_player(from_username) else {
            return Err("Associated player not found".to_string());
        };
        if from_player.is_gagged {
            return Err("You are gagged and cannot send messages".to_string());
        }
        let Some(to_client_id) = self.client_service.get_associated_client(to_username) else {
            return Ok(());
        };
        let msg = ServerMessage::ChatMessage {
            from: from_username.clone(),
            message: message.censor(),
            source: ChatMessageSource::Private,
        };
        self.client_service.try_protocol_send(&to_client_id, &msg);
        Ok(())
    }
}
