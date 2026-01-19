use std::sync::Arc;

use crate::{
    domain::{
        AccountId,
        chat::{ChatRoomService, ContentPolicy},
    },
    ports::{
        connection::AccountConnectionPort,
        notification::{ListenerMessage, ListenerNotificationPort},
    },
};

#[async_trait::async_trait]
pub trait ChatMessageUseCase {
    async fn send_message(&self, from: &AccountId, target: MessageTarget, message: &str);
}

//TODO: get rid of arbitrary rooms and introduce "match" contexts etc.

#[derive(Clone, Debug)]
pub enum MessageTarget {
    Private(AccountId),
    Room(String),
    Global,
}

pub struct ChatMessageUseCaseImpl<
    L: ListenerNotificationPort,
    P: AccountConnectionPort,
    C: ChatRoomService,
    Co: ContentPolicy,
> {
    listener_notification_port: Arc<L>,
    player_connection_port: Arc<P>,
    chat_room_service: Arc<C>,
    content_policy: Arc<Co>,
}

impl<L: ListenerNotificationPort, P: AccountConnectionPort, C: ChatRoomService, Co: ContentPolicy>
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
    P: AccountConnectionPort + Send + Sync + 'static,
    C: ChatRoomService + Send + Sync + 'static,
    Co: ContentPolicy + Send + Sync + 'static,
> ChatMessageUseCase for ChatMessageUseCaseImpl<L, P, C, Co>
{
    async fn send_message(
        &self,
        from_account_id: &AccountId,
        target: MessageTarget,
        message: &str,
    ) {
        let filtered_message = self.content_policy.filter_message(&message);
        match target {
            MessageTarget::Private(to_account_id) => {
                let (from_account_connection, to_account_connection) = futures::join!(
                    self.player_connection_port
                        .get_connection_id(from_account_id),
                    self.player_connection_port
                        .get_connection_id(&to_account_id)
                );
                let msg = ListenerMessage::ChatMessage {
                    from_account_id: from_account_id.clone(),
                    message: filtered_message.clone(),
                    target: MessageTarget::Private(to_account_id.clone()),
                };
                if let Some(connection_id) = from_account_connection {
                    self.listener_notification_port
                        .notify_listener(connection_id, &msg);
                }
                if let Some(connection_id) = to_account_connection {
                    self.listener_notification_port
                        .notify_listener(connection_id, &msg);
                }
            }
            MessageTarget::Global => {
                let msg = ListenerMessage::ChatMessage {
                    from_account_id: from_account_id.clone(),
                    message: filtered_message.clone(),
                    target: MessageTarget::Global,
                };
                self.listener_notification_port.notify_all(&msg);
            }
            MessageTarget::Room(room_name) => {
                let listeners_in_room = self.chat_room_service.get_listeners_in_room(&room_name);
                let msg = ListenerMessage::ChatMessage {
                    from_account_id: from_account_id.clone(),
                    message: filtered_message.clone(),
                    target: MessageTarget::Room(room_name),
                };
                self.listener_notification_port
                    .notify_listeners(&listeners_in_room, &msg);
            }
        }
    }
}
