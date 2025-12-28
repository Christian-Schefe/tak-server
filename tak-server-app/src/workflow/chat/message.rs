use std::sync::Arc;

use crate::{
    domain::{
        PlayerId,
        chat::{ChatRoomService, ContentPolicy},
    },
    ports::{
        connection::PlayerConnectionPort,
        notification::{ChatMessageSource, ListenerMessage, ListenerNotificationPort},
    },
};

#[async_trait::async_trait]
pub trait ChatMessageUseCase {
    async fn send_private_message(
        &self,
        from_player_id: PlayerId,
        to_player_id: PlayerId,
        message: &str,
    ) -> String;
    async fn send_global_message(&self, from_player_id: PlayerId, message: &str);
    async fn send_room_message(&self, from_player_id: PlayerId, room_name: &String, message: &str);
}

pub struct ChatMessageUseCaseImpl<
    L: ListenerNotificationPort,
    P: PlayerConnectionPort,
    C: ChatRoomService,
    Co: ContentPolicy,
> {
    listener_notification_port: Arc<L>,
    player_connection_port: Arc<P>,
    chat_room_service: Arc<C>,
    content_policy: Arc<Co>,
}

impl<L: ListenerNotificationPort, P: PlayerConnectionPort, C: ChatRoomService, Co: ContentPolicy>
    ChatMessageUseCaseImpl<L, P, C, Co>
{
    pub fn new(
        listener_notification_port: Arc<L>,
        player_connection_port: Arc<P>,
        chat_room_service: Arc<C>,
        content_policy: Arc<Co>,
    ) -> Self {
        Self {
            listener_notification_port,
            player_connection_port,
            chat_room_service,
            content_policy,
        }
    }
}

#[async_trait::async_trait]
impl<
    L: ListenerNotificationPort + Send + Sync + 'static,
    P: PlayerConnectionPort + Send + Sync + 'static,
    C: ChatRoomService + Send + Sync + 'static,
    Co: ContentPolicy + Send + Sync + 'static,
> ChatMessageUseCase for ChatMessageUseCaseImpl<L, P, C, Co>
{
    async fn send_private_message(
        &self,
        from_player_id: PlayerId,
        to_player_id: PlayerId,
        message: &str,
    ) -> String {
        let to_player_connection = self
            .player_connection_port
            .get_connection_id(to_player_id)
            .await;
        let filtered_message = self.content_policy.filter_message(&message);
        if let Some(connection_id) = to_player_connection {
            let msg = ListenerMessage::ChatMessage {
                from_player_id,
                message: filtered_message.clone(),
                source: ChatMessageSource::Private,
            };
            self.listener_notification_port
                .notify_listener(connection_id, msg);
        }
        filtered_message
    }

    async fn send_global_message(&self, from_player_id: PlayerId, message: &str) {
        let filtered_message = self.content_policy.filter_message(&message);
        let msg = ListenerMessage::ChatMessage {
            from_player_id,
            message: filtered_message,
            source: ChatMessageSource::Global,
        };
        self.listener_notification_port.notify_all(msg);
    }

    async fn send_room_message(&self, from_player_id: PlayerId, room_name: &String, message: &str) {
        let filtered_message = self.content_policy.filter_message(&message);
        let players_in_room = self.chat_room_service.get_listeners_in_room(room_name);
        let msg = ListenerMessage::ChatMessage {
            from_player_id,
            message: filtered_message,
            source: ChatMessageSource::Room(room_name.to_string()),
        };
        self.listener_notification_port
            .notify_listeners(&players_in_room, msg);
    }
}
