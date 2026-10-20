use crate::{
    PortsCollection,
    ports::{
        chat::ChatRepository,
        connection::UserConnectionPort,
        notification::{ListenerMessage, ListenerNotificationPort},
        transaction::{RepoError, TransactionManager},
    },
    workflow::chat::ChatMessageView,
};
use std::sync::Arc;
use tak_domain::{
    ChatMessageId, UserId,
    chat::{ChatConversation, ChatMessage, ChatRoomService, ContentPolicy},
};

#[async_trait::async_trait]
pub trait ChatMessageUseCase {
    async fn send_message(
        &self,
        from: UserId,
        conversation: &ChatConversation,
        message: &str,
    ) -> Result<(), ChatSendMessageError>;
    async fn get_messages(
        &self,
        conversation: &ChatConversation,
        before: Option<ChatMessageId>,
        limit: usize,
    ) -> Result<Vec<ChatMessageView>, ()>;
}

pub enum ChatSendMessageError {
    NotAllowed(String),
    RepositoryError,
}

pub struct ChatMessageUseCaseImpl<
    Ports: PortsCollection,
    L: ListenerNotificationPort,
    P: UserConnectionPort,
    C: ChatRoomService,
    Co: ContentPolicy,
> {
    ports: Arc<Ports>,
    listener_notification_port: Arc<L>,
    player_connection_port: Arc<P>,
    chat_room_service: Arc<C>,
    content_policy: Arc<Co>,
}

impl<
    Ports: PortsCollection,
    L: ListenerNotificationPort,
    P: UserConnectionPort,
    C: ChatRoomService,
    Co: ContentPolicy,
> ChatMessageUseCaseImpl<Ports, L, P, C, Co>
{
    pub fn new(
        ports: Arc<Ports>,
        listener_notification_port: Arc<L>,
        player_connection_port: Arc<P>,
        chat_room_service: Arc<C>,
        content_policy: Arc<Co>,
    ) -> Self {
        Self {
            ports,
            listener_notification_port,
            player_connection_port,
            chat_room_service,
            content_policy,
        }
    }
}

#[async_trait::async_trait]
impl<
    Ports: PortsCollection + Send + Sync + 'static,
    L: ListenerNotificationPort + Send + Sync + 'static,
    P: UserConnectionPort + Send + Sync + 'static,
    C: ChatRoomService + Send + Sync + 'static,
    Co: ContentPolicy + Send + Sync + 'static,
> ChatMessageUseCase for ChatMessageUseCaseImpl<Ports, L, P, C, Co>
{
    async fn send_message(
        &self,
        from_user_id: UserId,
        conversation: &ChatConversation,
        message: &str,
    ) -> Result<(), ChatSendMessageError> {
        let is_allowed_to_send = match conversation {
            ChatConversation::Private { user_ids } => {
                user_ids.0 == from_user_id || user_ids.1 == from_user_id
            }
            ChatConversation::Room { room_name: _ } => true,
            ChatConversation::Global => true,
        };
        if !is_allowed_to_send {
            return Err(ChatSendMessageError::NotAllowed(
                "You are not allowed to send a message in this conversation".to_string(),
            ));
        }

        let filtered_message = match self.content_policy.filter_message(&message) {
            Ok(msg) => msg,
            Err(reason) => return Err(ChatSendMessageError::NotAllowed(reason)),
        };

        let chat_msg = ChatMessage {
            date: chrono::Utc::now(),
            sender: from_user_id.clone(),
            message: filtered_message.clone(),
        };

        let ctx = self.ports.transaction_manager().simple();
        let msg_id = match self
            .ports
            .chat_repository()
            .save_message(&ctx, conversation.clone(), chat_msg.clone())
            .await
        {
            Ok(id) => id,
            Err(RepoError(e)) => {
                tracing::error!("Failed to save chat message: {}", e);
                return Err(ChatSendMessageError::RepositoryError);
            }
        };

        let msg = ListenerMessage::ChatMessage {
            message: ChatMessageView::from(msg_id, chat_msg),
            conversation: conversation.clone(),
        };

        match conversation {
            ChatConversation::Private { user_ids } => {
                let member_listener_futures =
                    [&user_ids.0, &user_ids.1]
                        .into_iter()
                        .map(|to_user_id| async move {
                            self.player_connection_port
                                .get_connection_id(*to_user_id)
                                .await
                        });
                let member_connections = futures::future::join_all(member_listener_futures).await;
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
    ) -> Result<Vec<ChatMessageView>, ()> {
        let ctx = self.ports.transaction_manager().simple();
        match self
            .ports
            .chat_repository()
            .get_messages(&ctx, conversation.clone(), before, limit)
            .await
        {
            Ok(messages) => Ok(messages
                .into_iter()
                .map(|(id, msg)| ChatMessageView::from(id, msg))
                .collect()),
            Err(RepoError(e)) => {
                tracing::error!("Failed to retrieve chat messages: {}", e);
                Err(())
            }
        }
    }
}
