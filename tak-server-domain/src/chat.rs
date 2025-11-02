use std::sync::Arc;

use rustrict::CensorStr;

use crate::{
    ServiceError, ServiceResult,
    game::SpectatorId,
    player::{PlayerService, PlayerUsername},
    transport::{ChatMessageSource, ServerMessage, TransportService},
    util::ManyManyDashMap,
};

pub type ArcChatService = Arc<Box<dyn ChatService + Send + Sync>>;
pub trait ChatService {
    fn join_room(&self, spectator: &SpectatorId, room_name: &String) -> ServiceResult<()>;
    fn leave_room(&self, spectator: &SpectatorId, room_name: &String) -> ServiceResult<()>;
    fn leave_all_rooms(&self, spectator: &SpectatorId) -> ServiceResult<()>;
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
    client_service: Arc<Box<dyn TransportService + Send + Sync>>,
    player_service: Arc<Box<dyn PlayerService + Send + Sync>>,
    chat_rooms: Arc<ManyManyDashMap<String, SpectatorId>>,
}

impl ChatServiceImpl {
    pub fn new(
        client_service: Arc<Box<dyn TransportService + Send + Sync>>,
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
    fn join_room(&self, spectator: &SpectatorId, room_name: &String) -> ServiceResult<()> {
        self.chat_rooms.insert(room_name.to_string(), *spectator);
        let msg = ServerMessage::RoomMembership {
            room: room_name.to_string(),
            joined: true,
        };
        self.client_service.try_spectator_send(spectator, &msg);
        Ok(())
    }

    fn leave_room(&self, spectator: &SpectatorId, room_name: &String) -> ServiceResult<()> {
        self.chat_rooms.remove(room_name, spectator);
        let msg = ServerMessage::RoomMembership {
            room: room_name.to_string(),
            joined: false,
        };
        self.client_service.try_spectator_send(spectator, &msg);
        Ok(())
    }

    fn leave_all_rooms(&self, spectator: &SpectatorId) -> ServiceResult<()> {
        self.chat_rooms.remove_value(spectator);
        Ok(())
    }

    fn send_message_to_all(&self, username: &PlayerUsername, message: &str) -> ServiceResult<()> {
        let player = self.player_service.fetch_player_data(username)?;
        if player.is_gagged {
            return ServiceError::forbidden("You are gagged and cannot send messages");
        }
        let msg = ServerMessage::ChatMessage {
            from: username.clone(),
            message: message.censor(),
            source: ChatMessageSource::Global,
        };
        self.client_service.try_player_broadcast(&msg);
        Ok(())
    }

    fn send_message_to_room(
        &self,
        username: &PlayerUsername,
        room_name: &String,
        message: &str,
    ) -> ServiceResult<()> {
        let player = self.player_service.fetch_player_data(&username)?;
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
            .try_spectator_multicast(&participants, &msg);

        Ok(())
    }

    fn send_message_to_player(
        &self,
        from_username: &PlayerUsername,
        to_username: &PlayerUsername,
        message: &str,
    ) -> ServiceResult<String> {
        let from_player = self.player_service.fetch_player_data(from_username)?;
        if from_player.is_gagged {
            return ServiceError::forbidden("You are gagged and cannot send messages");
        }
        let censored_message = message.censor();

        let msg = ServerMessage::ChatMessage {
            from: from_username.clone(),
            message: censored_message.clone(),
            source: ChatMessageSource::Private,
        };
        self.client_service.try_player_send(&to_username, &msg);
        Ok(censored_message)
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::{player::MockPlayerService, transport::MockTransportService};

    use super::*;

    #[test]
    fn test_private_message() {
        let mock_client_service = MockTransportService::default();
        let mock_player_service = MockPlayerService::default();
        let chat_service = ChatServiceImpl::new(
            Arc::new(Box::new(mock_client_service.clone())),
            Arc::new(Box::new(mock_player_service.clone())),
        );

        assert_eq!(
            chat_service
                .send_message_to_player(
                    &"test_admin".to_string(),
                    &"test_user".to_string(),
                    "Hello!",
                )
                .ok(),
            Some("Hello!".to_string())
        );
        let messages = mock_client_service.get_messages();
        assert_eq!(messages.len(), 1);
        assert!(
            matches!(&messages[0], (id, ServerMessage::ChatMessage { from, message, source }) if *id == "test_user" && from == "test_admin" && message == "Hello!" && *source == ChatMessageSource::Private)
        );

        assert!(matches!(
            chat_service.send_message_to_player(
                &"test_gagged".to_string(),
                &"test_user".to_string(),
                "Hello!",
            ),
            Err(ServiceError::Forbidden(..))
        ));
    }

    #[test]
    fn test_room_message() {
        let mock_client_service = MockTransportService::default();
        let mock_player_service = MockPlayerService::default();
        let chat_service = ChatServiceImpl::new(
            Arc::new(Box::new(mock_client_service.clone())),
            Arc::new(Box::new(mock_player_service.clone())),
        );

        let p1 = Uuid::new_v4();
        let p2 = Uuid::new_v4();

        chat_service.join_room(&p1, &"room".to_string()).unwrap();
        chat_service.join_room(&p2, &"room".to_string()).unwrap();
        let messages = mock_client_service.get_spectator_messages();
        assert_eq!(messages.len(), 2);

        assert!(
            chat_service
                .send_message_to_room(&"test_admin".to_string(), &"room".to_string(), "Hello!")
                .is_ok()
        );
        let messages = mock_client_service.get_spectator_messages();
        assert_eq!(messages.len(), 4);
        assert!(
            matches!(&messages[2].1,  ServerMessage::ChatMessage { from, message, source: ChatMessageSource::Room{ name } } if from == "test_admin" && message == "Hello!" && *name ==  "room".to_string()),
        );
        assert!(
            matches!(&messages[3].1,  ServerMessage::ChatMessage { from, message, source: ChatMessageSource::Room{ name } } if from == "test_admin" && message == "Hello!" && *name ==  "room".to_string())
        );
        assert!(
            messages[2].0 != messages[3].0
                && (messages[2].0 == p1 || messages[2].0 == p2)
                && (messages[3].0 == p1 || messages[3].0 == p2)
        );

        assert!(matches!(
            chat_service.send_message_to_player(
                &"test_gagged".to_string(),
                &"test_user".to_string(),
                "Hello!",
            ),
            Err(ServiceError::Forbidden(..))
        ));
    }

    #[test]
    fn test_global_message() {
        let mock_client_service = MockTransportService::default();
        let mock_player_service = MockPlayerService::default();
        let chat_service = ChatServiceImpl::new(
            Arc::new(Box::new(mock_client_service.clone())),
            Arc::new(Box::new(mock_player_service.clone())),
        );

        assert!(
            chat_service
                .send_message_to_all(&"test_admin".to_string(), "Hello!")
                .is_ok()
        );
        let messages = mock_client_service.get_broadcasts();
        assert_eq!(messages.len(), 1);
        assert!(
            matches!(&messages[0],  ServerMessage::ChatMessage { from, message, source: ChatMessageSource::Global } if from == "test_admin" && message == "Hello!"),
        );

        assert!(matches!(
            chat_service.send_message_to_all(&"test_gagged".to_string(), "Hello!",),
            Err(ServiceError::Forbidden(..))
        ));
    }
}
