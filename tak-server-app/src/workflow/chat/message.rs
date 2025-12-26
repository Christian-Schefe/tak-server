use std::sync::Arc;

use crate::{
    domain::{
        PlayerId,
        chat::{ChatRoomService, ContentPolicy},
        player::PlayerRepository,
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
    ) -> Result<String, ChatMessageError>;
    async fn send_global_message(
        &self,
        from_player_id: PlayerId,
        message: &str,
    ) -> Result<(), ChatMessageError>;
    async fn send_room_message(
        &self,
        from_player_id: PlayerId,
        room_name: &String,
        message: &str,
    ) -> Result<(), ChatMessageError>;
}

pub enum ChatMessageError {
    FailedToRetrievePlayer,
    PlayerSilenced,
}

pub struct ChatMessageUseCaseImpl<
    L: ListenerNotificationPort,
    P: PlayerConnectionPort,
    C: ChatRoomService,
    Co: ContentPolicy,
    PR: PlayerRepository,
> {
    listener_notification_port: Arc<L>,
    player_connection_port: Arc<P>,
    chat_room_service: Arc<C>,
    content_policy: Arc<Co>,
    player_repo: Arc<PR>,
}

impl<
    L: ListenerNotificationPort,
    P: PlayerConnectionPort,
    C: ChatRoomService,
    Co: ContentPolicy,
    PR: PlayerRepository,
> ChatMessageUseCaseImpl<L, P, C, Co, PR>
{
    pub fn new(
        listener_notification_port: Arc<L>,
        player_connection_port: Arc<P>,
        chat_room_service: Arc<C>,
        content_policy: Arc<Co>,
        player_repo: Arc<PR>,
    ) -> Self {
        Self {
            listener_notification_port,
            player_connection_port,
            chat_room_service,
            content_policy,
            player_repo,
        }
    }

    async fn filter_message(
        &self,
        player_id: PlayerId,
        message: &str,
    ) -> Result<String, ChatMessageError> {
        match self.player_repo.get_player(player_id).await {
            Ok(player) => {
                if player.is_silenced {
                    return Err(ChatMessageError::PlayerSilenced);
                }
            }
            Err(_) => return Err(ChatMessageError::FailedToRetrievePlayer),
        }
        let filtered_message = self.content_policy.filter_message(&message);
        Ok(filtered_message)
    }
}

#[async_trait::async_trait]
impl<
    L: ListenerNotificationPort + Send + Sync + 'static,
    P: PlayerConnectionPort + Send + Sync + 'static,
    C: ChatRoomService + Send + Sync + 'static,
    Co: ContentPolicy + Send + Sync + 'static,
    PR: PlayerRepository + Send + Sync + 'static,
> ChatMessageUseCase for ChatMessageUseCaseImpl<L, P, C, Co, PR>
{
    async fn send_private_message(
        &self,
        from_player_id: PlayerId,
        to_player_id: PlayerId,
        message: &str,
    ) -> Result<String, ChatMessageError> {
        let to_player_connection = self.player_connection_port.get_connection_id(to_player_id);
        let filtered_message = self.filter_message(from_player_id, message).await?;
        if let Some(connection_id) = to_player_connection {
            let msg = ListenerMessage::ChatMessage {
                from_player_id,
                message: filtered_message.clone(),
                source: ChatMessageSource::Private,
            };
            self.listener_notification_port
                .notify_listener(connection_id, msg);
        }
        Ok(filtered_message)
    }

    async fn send_global_message(
        &self,
        from_player_id: PlayerId,
        message: &str,
    ) -> Result<(), ChatMessageError> {
        let filtered_message = self.filter_message(from_player_id, message).await?;
        let msg = ListenerMessage::ChatMessage {
            from_player_id,
            message: filtered_message,
            source: ChatMessageSource::Global,
        };
        self.listener_notification_port.notify_all(msg);
        Ok(())
    }

    async fn send_room_message(
        &self,
        from_player_id: PlayerId,
        room_name: &String,
        message: &str,
    ) -> Result<(), ChatMessageError> {
        let filtered_message = self.filter_message(from_player_id, message).await?;
        let players_in_room = self.chat_room_service.get_listeners_in_room(room_name);
        let msg = ListenerMessage::ChatMessage {
            from_player_id,
            message: filtered_message,
            source: ChatMessageSource::Room(room_name.to_string()),
        };
        self.listener_notification_port
            .notify_listeners(&players_in_room, msg);
        Ok(())
    }
}
