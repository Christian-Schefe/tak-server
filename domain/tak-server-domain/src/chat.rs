use std::sync::Arc;

use rustrict::CensorStr;

use crate::{
    ServiceError, ServiceResult,
    player::{ArcPlayerService, PlayerUsername},
    transport::{
        ArcPlayerConnectionService, ArcTransportService, ChatMessageSource, ListenerId,
        ServerMessage, do_player_broadcast, do_player_send,
    },
    util::ManyManyDashMap,
};

pub type ArcChatService = Arc<Box<dyn ChatService + Send + Sync>>;

#[async_trait::async_trait]
pub trait ChatService {
    async fn join_room(&self, id: ListenerId, room_name: &String) -> ServiceResult<()>;
    async fn leave_room(&self, id: ListenerId, room_name: &String) -> ServiceResult<()>;
    fn leave_all_rooms_quiet(&self, id: ListenerId) -> ServiceResult<()>;
    async fn send_message_to_all(
        &self,
        username: &PlayerUsername,
        message: &str,
    ) -> ServiceResult<()>;
    async fn send_message_to_room(
        &self,
        username: &PlayerUsername,
        room_name: &String,
        message: &str,
    ) -> ServiceResult<()>;
    async fn send_message_to_player(
        &self,
        from_username: &PlayerUsername,
        to_username: &PlayerUsername,
        message: &str,
    ) -> ServiceResult<String>;
}

pub struct ChatServiceImpl {
    transport_service: ArcTransportService,
    player_connection_service: ArcPlayerConnectionService,
    player_service: ArcPlayerService,
    chat_rooms: Arc<ManyManyDashMap<String, ListenerId>>,
}

impl ChatServiceImpl {
    pub fn new(
        transport_service: ArcTransportService,
        player_connection_service: ArcPlayerConnectionService,
        player_service: ArcPlayerService,
    ) -> Self {
        Self {
            transport_service,
            player_connection_service,
            player_service,
            chat_rooms: Arc::new(ManyManyDashMap::new()),
        }
    }
}

#[async_trait::async_trait]
impl ChatService for ChatServiceImpl {
    async fn join_room(&self, id: ListenerId, room_name: &String) -> ServiceResult<()> {
        self.chat_rooms.insert(room_name.to_string(), id);
        let msg = ServerMessage::RoomMembership {
            room: room_name.to_string(),
            joined: true,
        };
        self.transport_service.try_listener_send(id, &msg).await;
        Ok(())
    }

    async fn leave_room(&self, id: ListenerId, room_name: &String) -> ServiceResult<()> {
        self.chat_rooms.remove(room_name, &id);
        let msg = ServerMessage::RoomMembership {
            room: room_name.to_string(),
            joined: false,
        };
        self.transport_service.try_listener_send(id, &msg).await;
        Ok(())
    }

    fn leave_all_rooms_quiet(&self, id: ListenerId) -> ServiceResult<()> {
        self.chat_rooms.remove_value(&id);
        Ok(())
    }

    async fn send_message_to_all(
        &self,
        username: &PlayerUsername,
        message: &str,
    ) -> ServiceResult<()> {
        let player = self.player_service.get_player(username).await?;
        if player.flags.is_silenced {
            do_player_send(
                &self.player_connection_service,
                &self.transport_service,
                username,
                &ServerMessage::ChatMessage {
                    from: username.clone(),
                    message: "<Server: You have been silenced for inappropriate chat behavior.>"
                        .to_string(),
                    source: ChatMessageSource::Global,
                },
            )
            .await;
            return ServiceError::forbidden("You are silenced and cannot send messages");
        }
        let msg = ServerMessage::ChatMessage {
            from: username.clone(),
            message: message.censor(),
            source: ChatMessageSource::Global,
        };
        do_player_broadcast(
            &self.player_connection_service,
            &self.transport_service,
            &msg,
        )
        .await;
        Ok(())
    }

    async fn send_message_to_room(
        &self,
        username: &PlayerUsername,
        room_name: &String,
        message: &str,
    ) -> ServiceResult<()> {
        let player = self.player_service.get_player(&username).await?;
        if player.flags.is_silenced {
            return ServiceError::forbidden("You are silenced and cannot send messages");
        }
        let participants = self.chat_rooms.get_by_key(room_name);
        let msg = ServerMessage::ChatMessage {
            from: username.clone(),
            message: message.censor(),
            source: ChatMessageSource::Room {
                name: room_name.to_string(),
            },
        };
        self.transport_service
            .try_listener_multicast(&participants, &msg)
            .await;

        Ok(())
    }

    async fn send_message_to_player(
        &self,
        from_username: &PlayerUsername,
        to_username: &PlayerUsername,
        message: &str,
    ) -> ServiceResult<String> {
        let from_player = self.player_service.get_player(from_username).await?;
        if from_player.flags.is_silenced {
            return ServiceError::forbidden("You are silenced and cannot send messages");
        }
        let censored_message = message.censor();

        let msg = ServerMessage::ChatMessage {
            from: from_username.clone(),
            message: censored_message.clone(),
            source: ChatMessageSource::Private,
        };
        do_player_send(
            &self.player_connection_service,
            &self.transport_service,
            to_username,
            &msg,
        )
        .await;
        Ok(censored_message)
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        player::MockPlayerService,
        transport::{MockPlayerConnectionService, MockTransportService, PlayerConnectionService},
    };

    use super::*;

    #[tokio::test]
    async fn test_private_message() {
        let mock_client_service = MockTransportService::default();
        let mock_player_connection_service = MockPlayerConnectionService::new();
        let mock_player_service = MockPlayerService::default();
        let chat_service = ChatServiceImpl::new(
            Arc::new(Box::new(mock_client_service.clone())),
            Arc::new(Box::new(mock_player_connection_service.clone())),
            Arc::new(Box::new(mock_player_service.clone())),
        );

        let test_user_id = ListenerId::new();
        let test_admin_id = ListenerId::new();

        mock_player_connection_service
            .on_player_connected(test_user_id, &"test_user".to_string())
            .await;
        mock_player_connection_service
            .on_player_connected(test_admin_id, &"test_admin".to_string())
            .await;

        assert_eq!(
            chat_service
                .send_message_to_player(
                    &"test_admin".to_string(),
                    &"test_user".to_string(),
                    "Hello!",
                )
                .await
                .ok(),
            Some("Hello!".to_string())
        );
        let messages = mock_client_service.get_messages();
        assert_eq!(messages.len(), 1);
        assert!(
            matches!(&messages[0], (id, ServerMessage::ChatMessage { from, message, source }) if *id == test_user_id && from == "test_admin" && message == "Hello!" && *source == ChatMessageSource::Private)
        );

        assert!(matches!(
            chat_service
                .send_message_to_player(
                    &"test_silenced".to_string(),
                    &"test_user".to_string(),
                    "Hello!",
                )
                .await,
            Err(ServiceError::Forbidden(..))
        ));
    }

    #[tokio::test]
    async fn test_room_message() {
        let mock_client_service = MockTransportService::default();
        let mock_player_connection_service = MockPlayerConnectionService::new();
        let mock_player_service = MockPlayerService::default();
        let chat_service = ChatServiceImpl::new(
            Arc::new(Box::new(mock_client_service.clone())),
            Arc::new(Box::new(mock_player_connection_service.clone())),
            Arc::new(Box::new(mock_player_service.clone())),
        );

        let test_user = ListenerId::new();
        let test_admin = ListenerId::new();

        mock_player_connection_service
            .on_player_connected(test_user, &"test_user".to_string())
            .await;
        mock_player_connection_service
            .on_player_connected(test_admin, &"test_admin".to_string())
            .await;

        chat_service
            .join_room(test_user, &"room".to_string())
            .await
            .unwrap();
        chat_service
            .join_room(test_admin, &"room".to_string())
            .await
            .unwrap();
        let messages = mock_client_service.get_messages();
        assert_eq!(messages.len(), 2);

        assert!(
            chat_service
                .send_message_to_room(&"test_admin".to_string(), &"room".to_string(), "Hello!")
                .await
                .is_ok()
        );
        let messages = mock_client_service.get_messages();
        assert_eq!(messages.len(), 4);
        assert!(
            matches!(&messages[2].1,  ServerMessage::ChatMessage { from, message, source: ChatMessageSource::Room{ name } } if from == "test_admin" && message == "Hello!" && *name ==  "room".to_string()),
        );
        assert!(
            matches!(&messages[3].1,  ServerMessage::ChatMessage { from, message, source: ChatMessageSource::Room{ name } } if from == "test_admin" && message == "Hello!" && *name ==  "room".to_string())
        );
        assert!(
            messages[2].0 != messages[3].0
                && (messages[2].0 == test_user || messages[2].0 == test_admin)
                && (messages[3].0 == test_user || messages[3].0 == test_admin)
        );

        assert!(matches!(
            chat_service
                .send_message_to_player(
                    &"test_silenced".to_string(),
                    &"test_user".to_string(),
                    "Hello!",
                )
                .await,
            Err(ServiceError::Forbidden(..))
        ));
    }

    #[tokio::test]
    async fn test_global_message() {
        let mock_client_service = MockTransportService::default();
        let mock_player_connection_service = MockPlayerConnectionService::new();
        let mock_player_service = MockPlayerService::default();
        let chat_service = ChatServiceImpl::new(
            Arc::new(Box::new(mock_client_service.clone())),
            Arc::new(Box::new(mock_player_connection_service.clone())),
            Arc::new(Box::new(mock_player_service.clone())),
        );

        let test_admin = ListenerId::new();
        let test_silenced = ListenerId::new();

        mock_player_connection_service
            .on_player_connected(test_admin, &"test_admin".to_string())
            .await;
        mock_player_connection_service
            .on_player_connected(test_silenced, &"test_silenced".to_string())
            .await;

        assert!(
            chat_service
                .send_message_to_all(&"test_admin".to_string(), "Hello!")
                .await
                .is_ok()
        );
        let messages = mock_client_service.get_messages();
        assert_eq!(messages.len(), 2);
        assert!(
            matches!(&messages[0].1,  ServerMessage::ChatMessage { from, message, source: ChatMessageSource::Global } if from == "test_admin" && message == "Hello!"),
        );
        assert!(
            matches!(&messages[1].1,  ServerMessage::ChatMessage { from, message, source: ChatMessageSource::Global } if from == "test_admin" && message == "Hello!"),
        );

        assert!(matches!(
            chat_service
                .send_message_to_all(&"test_silenced".to_string(), "Hello!",)
                .await,
            Err(ServiceError::Forbidden(..))
        ));
    }
}
