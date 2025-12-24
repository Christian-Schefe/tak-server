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

pub trait ChatMessageUseCase {
    fn send_private_message(
        &self,
        from_player_id: PlayerId,
        to_player_id: PlayerId,
        message: String,
    ) -> Result<(), ChatMessageError>;
    fn send_global_message(
        &self,
        from_player_id: PlayerId,
        message: String,
    ) -> Result<(), ChatMessageError>;
    fn send_room_message(
        &self,
        from_player_id: PlayerId,
        room_name: String,
        message: String,
    ) -> Result<(), ChatMessageError>;
}

pub enum ChatMessageError {
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

    fn filter_message(
        &self,
        player_id: PlayerId,
        message: String,
    ) -> Result<String, ChatMessageError> {
        if self
            .player_repo
            .get_player(player_id)
            .is_some_and(|player| player.is_silenced)
        {
            return Err(ChatMessageError::PlayerSilenced);
        }
        let filtered_message = self.content_policy.filter_message(&message);
        Ok(filtered_message)
    }
}

impl<
    L: ListenerNotificationPort,
    P: PlayerConnectionPort,
    C: ChatRoomService,
    Co: ContentPolicy,
    PR: PlayerRepository,
> ChatMessageUseCase for ChatMessageUseCaseImpl<L, P, C, Co, PR>
{
    fn send_private_message(
        &self,
        from_player_id: PlayerId,
        to_player_id: PlayerId,
        message: String,
    ) -> Result<(), ChatMessageError> {
        let to_player_connection = self.player_connection_port.get_connection_id(to_player_id);
        let filtered_message = self.filter_message(from_player_id, message)?;
        if let Some(connection_id) = to_player_connection {
            let msg = ListenerMessage::ChatMessage {
                from_player_id,
                message: filtered_message,
                source: ChatMessageSource::Private,
            };
            self.listener_notification_port
                .notify_listener(connection_id, msg);
        }
        Ok(())
    }

    fn send_global_message(
        &self,
        from_player_id: PlayerId,
        message: String,
    ) -> Result<(), ChatMessageError> {
        let filtered_message = self.filter_message(from_player_id, message)?;
        let msg = ListenerMessage::ChatMessage {
            from_player_id,
            message: filtered_message,
            source: ChatMessageSource::Global,
        };
        self.listener_notification_port.notify_all(msg);
        Ok(())
    }

    fn send_room_message(
        &self,
        from_player_id: PlayerId,
        room_name: String,
        message: String,
    ) -> Result<(), ChatMessageError> {
        let filtered_message = self.filter_message(from_player_id, message)?;
        let players_in_room = self.chat_room_service.get_listeners_in_room(&room_name);
        let msg = ListenerMessage::ChatMessage {
            from_player_id,
            message: filtered_message,
            source: ChatMessageSource::Room(room_name),
        };
        self.listener_notification_port
            .notify_listeners(&players_in_room, msg);
        Ok(())
    }
}
