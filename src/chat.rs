use std::sync::Arc;

use rustrict::CensorStr;

use crate::{
    ServiceError, ServiceResult,
    client::{ClientId, ClientService},
    player::{PlayerService, PlayerUsername},
    protocol::{ChatMessageSource, ServerMessage},
    util::ManyManyDashMap,
};

pub trait ChatService {
    fn join_room(&self, client_id: &ClientId, room_name: &String) -> ServiceResult<()>;
    fn leave_room(&self, client_id: &ClientId, room_name: &String) -> ServiceResult<()>;
    fn leave_all_rooms(&self, client_id: &ClientId) -> ServiceResult<()>;
    fn send_message_to_all(&self, username: &PlayerUsername, message: &str) -> ServiceResult<()>;
    fn send_message_to_room(
        &self,
        username: &PlayerUsername,
        room_name: &String,
        message: &str,
    ) -> ServiceResult<()>;
    fn send_message_to_player(
        &self,
        from_username: &PlayerUsername,
        to_username: &PlayerUsername,
        message: &str,
    ) -> ServiceResult<String>;
}

pub struct ChatServiceImpl {
    client_service: Arc<Box<dyn ClientService + Send + Sync>>,
    player_service: Arc<Box<dyn PlayerService + Send + Sync>>,
    chat_rooms: Arc<ManyManyDashMap<String, ClientId>>,
}

impl ChatServiceImpl {
    pub fn new(
        client_service: Arc<Box<dyn ClientService + Send + Sync>>,
        player_service: Arc<Box<dyn PlayerService + Send + Sync>>,
    ) -> Self {
        Self {
            client_service,
            player_service,
            chat_rooms: Arc::new(ManyManyDashMap::new()),
        }
    }
}

impl ChatService for ChatServiceImpl {
    fn join_room(&self, client_id: &ClientId, room_name: &String) -> ServiceResult<()> {
        self.chat_rooms.insert(room_name.to_string(), *client_id);
        let msg = ServerMessage::RoomMembership {
            room: room_name.to_string(),
            joined: true,
        };
        self.client_service.try_protocol_send(client_id, &msg);
        Ok(())
    }

    fn leave_room(&self, client_id: &ClientId, room_name: &String) -> ServiceResult<()> {
        self.chat_rooms.remove(room_name, client_id);
        let msg = ServerMessage::RoomMembership {
            room: room_name.to_string(),
            joined: false,
        };
        self.client_service.try_protocol_send(client_id, &msg);
        Ok(())
    }

    fn leave_all_rooms(&self, client_id: &ClientId) -> ServiceResult<()> {
        self.chat_rooms.remove_value(client_id);
        Ok(())
    }

    fn send_message_to_all(&self, username: &PlayerUsername, message: &str) -> ServiceResult<()> {
        let player = self.player_service.fetch_player(username)?;
        if player.is_gagged {
            return ServiceError::forbidden("You are gagged and cannot send messages");
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
        room_name: &String,
        message: &str,
    ) -> ServiceResult<()> {
        let player = self.player_service.fetch_player(&username)?;
        if player.is_gagged {
            return ServiceError::forbidden("You are gagged and cannot send messages");
        }
        let participants = self.chat_rooms.get_by_key(room_name);
        let msg = ServerMessage::ChatMessage {
            from: username.clone(),
            message: message.censor(),
            source: ChatMessageSource::Room {
                name: room_name.to_string(),
            },
        };
        self.client_service
            .try_protocol_multicast(&participants, &msg);

        Ok(())
    }

    fn send_message_to_player(
        &self,
        from_username: &PlayerUsername,
        to_username: &PlayerUsername,
        message: &str,
    ) -> ServiceResult<String> {
        let from_player = self.player_service.fetch_player(from_username)?;
        if from_player.is_gagged {
            return ServiceError::forbidden("You are gagged and cannot send messages");
        }
        let censored_message = message.censor();
        let Some(to_client_id) = self.client_service.get_associated_client(to_username) else {
            return Ok(censored_message);
        };
        let msg = ServerMessage::ChatMessage {
            from: from_username.clone(),
            message: censored_message.clone(),
            source: ChatMessageSource::Private,
        };
        self.client_service.try_protocol_send(&to_client_id, &msg);
        Ok(censored_message)
    }
}
