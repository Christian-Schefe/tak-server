use std::sync::Arc;

use crate::{
    domain::{
        PlayerId,
        account::AccountRepository,
        chat::{ChatRoomService, ContentPolicy},
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
    );
    fn send_global_message(&self, from_player_id: PlayerId, message: String);
    fn send_room_message(&self, from_player_id: PlayerId, room_name: String, message: String);
}

pub struct ChatMessageUseCaseImpl<
    L: ListenerNotificationPort,
    P: PlayerConnectionPort,
    C: ChatRoomService,
    Co: ContentPolicy,
    A: AccountRepository,
> {
    listener_notification_port: Arc<L>,
    player_connection_port: Arc<P>,
    chat_room_service: Arc<C>,
    content_policy: Arc<Co>,
    account_repo: Arc<A>,
}

impl<
    L: ListenerNotificationPort,
    P: PlayerConnectionPort,
    C: ChatRoomService,
    Co: ContentPolicy,
    A: AccountRepository,
> ChatMessageUseCaseImpl<L, P, C, Co, A>
{
    pub fn new(
        listener_notification_port: Arc<L>,
        player_connection_port: Arc<P>,
        chat_room_service: Arc<C>,
        content_policy: Arc<Co>,
        account_repo: Arc<A>,
    ) -> Self {
        Self {
            listener_notification_port,
            player_connection_port,
            chat_room_service,
            content_policy,
            account_repo,
        }
    }

    fn filter_message(&self, player_id: PlayerId, message: String) -> Option<String> {
        if self.account_repo.is_player_silenced(player_id) {
            return None;
        }
        let filtered_message = self.content_policy.filter_message(&message);
        Some(filtered_message)
    }
}

impl<
    L: ListenerNotificationPort,
    P: PlayerConnectionPort,
    C: ChatRoomService,
    Co: ContentPolicy,
    A: AccountRepository,
> ChatMessageUseCase for ChatMessageUseCaseImpl<L, P, C, Co, A>
{
    fn send_private_message(
        &self,
        from_player_id: PlayerId,
        to_player_id: PlayerId,
        message: String,
    ) {
        let to_player_connection = self.player_connection_port.get_connection_id(to_player_id);
        let Some(filtered_message) = self.filter_message(from_player_id, message) else {
            return;
        };
        if let Some(connection_id) = to_player_connection {
            let msg = ListenerMessage::ChatMessage {
                from_player_id,
                message: filtered_message,
                source: ChatMessageSource::Private,
            };
            self.listener_notification_port
                .notify_listener(connection_id, msg);
        }
    }

    fn send_global_message(&self, from_player_id: PlayerId, message: String) {
        let Some(filtered_message) = self.filter_message(from_player_id, message) else {
            return;
        };
        let msg = ListenerMessage::ChatMessage {
            from_player_id,
            message: filtered_message,
            source: ChatMessageSource::Global,
        };
        self.listener_notification_port.notify_all(msg);
    }

    fn send_room_message(&self, from_player_id: PlayerId, room_name: String, message: String) {
        let Some(filtered_message) = self.filter_message(from_player_id, message) else {
            return;
        };
        let players_in_room = self.chat_room_service.get_players_in_room(&room_name);
        let msg = ListenerMessage::ChatMessage {
            from_player_id,
            message: filtered_message,
            source: ChatMessageSource::Room(room_name),
        };
        self.listener_notification_port
            .notify_listeners(&players_in_room, msg);
    }
}
