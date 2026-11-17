use std::sync::Arc;

use tak_domain::{
    ChatMessageId, ListenerId, UserId,
    chat::{ChatConversation, ChatMessage, ContentPolicy, RustrictContentPolicy},
};
use terrors::OneOf;

use crate::{
    PortsCollection,
    errors::{AppResult, InsufficientPermissions, InternalError, InvalidChatMessage},
    ports::{
        chat::{ChatRepository, ChatRoomService},
        connection::UserConnectionPort,
        notification::{ListenerMessage, ListenerNotificationPort},
        transaction::TransactionManager,
    },
};

#[derive(Clone, Debug)]
pub struct ChatMessageView {
    pub id: ChatMessageId,
    pub date: chrono::DateTime<chrono::Utc>,
    pub sender: UserId,
    pub message: String,
}

impl ChatMessageView {
    fn from(id: ChatMessageId, message: ChatMessage) -> Self {
        Self {
            id,
            date: message.date,
            sender: message.sender,
            message: message.message,
        }
    }
}

#[async_trait::async_trait]
pub trait ChatUseCase {
    async fn send_message(
        &self,
        from: UserId,
        conversation: &ChatConversation,
        message: &str,
    ) -> AppResult<(), (InsufficientPermissions, InvalidChatMessage, InternalError)>;
    async fn get_messages(
        &self,
        conversation: &ChatConversation,
        before: Option<ChatMessageId>,
        limit: usize,
    ) -> AppResult<Vec<ChatMessageView>, (InternalError,)>;
    fn join_room(&self, room_name: &String, listener_id: ListenerId);
    fn leave_room(&self, room_name: &String, listener_id: ListenerId);
}

pub struct ChatUseCaseImpl<
    Ports: PortsCollection,
    L: ListenerNotificationPort,
    P: UserConnectionPort,
    C: ChatRoomService,
> {
    ports: Arc<Ports>,
    listener_notification_port: Arc<L>,
    player_connection_port: Arc<P>,
    chat_room_service: Arc<C>,
}

impl<Ports: PortsCollection, L: ListenerNotificationPort, P: UserConnectionPort, C: ChatRoomService>
    ChatUseCaseImpl<Ports, L, P, C>
{
    pub fn new(
        ports: Arc<Ports>,
        listener_notification_port: Arc<L>,
        player_connection_port: Arc<P>,
        chat_room_service: Arc<C>,
    ) -> Self {
        Self {
            ports,
            listener_notification_port,
            player_connection_port,
            chat_room_service,
        }
    }
}

#[async_trait::async_trait]
impl<
    Ports: PortsCollection + Send + Sync + 'static,
    L: ListenerNotificationPort + Send + Sync + 'static,
    P: UserConnectionPort + Send + Sync + 'static,
    C: ChatRoomService + Send + Sync + 'static,
> ChatUseCase for ChatUseCaseImpl<Ports, L, P, C>
{
    async fn send_message(
        &self,
        from_user_id: UserId,
        conversation: &ChatConversation,
        message: &str,
    ) -> AppResult<(), (InsufficientPermissions, InvalidChatMessage, InternalError)> {
        let is_allowed_to_send = match conversation {
            ChatConversation::Private { user_ids } => {
                user_ids.0 == from_user_id || user_ids.1 == from_user_id
            }
            ChatConversation::Room { room_name: _ } => true,
            ChatConversation::Global => true,
        };
        if !is_allowed_to_send {
            return Err(OneOf::new(InsufficientPermissions));
        }

        let filtered_message = match RustrictContentPolicy.filter_message(&message) {
            Ok(msg) => msg,
            Err(_reason) => return Err(OneOf::new(InvalidChatMessage)),
        };

        let chat_msg = ChatMessage {
            date: chrono::Utc::now(),
            sender: from_user_id.clone(),
            message: filtered_message.clone(),
        };

        let ctx = self.ports.transaction_manager().simple();
        let msg_id = self
            .ports
            .chat_repository()
            .save_message(&ctx, conversation.clone(), chat_msg.clone())
            .await
            .map_err(OneOf::broaden)?;

        let msg = ListenerMessage::ChatMessage {
            message: ChatMessageView::from(msg_id, chat_msg),
            conversation: conversation.clone(),
        };

        match conversation {
            ChatConversation::Private { user_ids } => {
                let member_connections = [&user_ids.0, &user_ids.1]
                    .into_iter()
                    .map(|to_user_id| self.player_connection_port.get_connection_id(*to_user_id));
                for member in member_connections {
                    if let Some(connection_id) = member {
                        self.listener_notification_port
                            .notify_listener(connection_id, &msg);
                    }
                }
            }
            ChatConversation::Global => {
                self.listener_notification_port.notify_all(&msg);
            }
            ChatConversation::Room { room_name } => {
                let listeners_in_room = self.chat_room_service.get_listeners_in_room(&room_name);
                self.listener_notification_port
                    .notify_listeners(&listeners_in_room, &msg);
            }
        }
        Ok(())
    }

    async fn get_messages(
        &self,
        conversation: &ChatConversation,
        before: Option<ChatMessageId>,
        limit: usize,
    ) -> AppResult<Vec<ChatMessageView>, (InternalError,)> {
        let ctx = self.ports.transaction_manager().simple();
        let messages = self
            .ports
            .chat_repository()
            .get_messages(&ctx, conversation.clone(), before, limit)
            .await
            .map_err(OneOf::broaden)?;
        Ok(messages
            .into_iter()
            .map(|(id, msg)| ChatMessageView::from(id, msg))
            .collect())
    }

    fn join_room(&self, room_name: &String, listener_id: ListenerId) {
        self.chat_room_service.join_room(room_name, listener_id);
    }

    fn leave_room(&self, room_name: &String, listener_id: ListenerId) {
        self.chat_room_service.leave_room(room_name, listener_id);
    }
}
