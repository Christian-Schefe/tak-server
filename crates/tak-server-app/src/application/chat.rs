use std::sync::Arc;

use better_errors::OneOf;
use tak_domain::{
    ChatMessageId, UserId,
    chat::{ChatConversation, ChatMessage, ContentPolicy, RustrictContentPolicy},
};

use crate::{
    PortsCollection,
    errors::{AppResult, InsufficientPermissions, InternalError, InvalidChatMessage},
    ports::notification::NotificationMessage,
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
}

pub struct ChatUseCaseImpl<Ports: PortsCollection> {
    ports: Arc<Ports>,
}

impl<Ports: PortsCollection> ChatUseCaseImpl<Ports> {
    pub fn new(ports: Arc<Ports>) -> Self {
        Self { ports }
    }
}

#[async_trait::async_trait]
impl<Ports: PortsCollection + Send + Sync + 'static> ChatUseCase for ChatUseCaseImpl<Ports> {
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

        let msg = NotificationMessage::ChatMessage {
            message: ChatMessageView::from(msg_id, chat_msg),
            conversation: conversation.clone(),
        };
        self.ports.notification_port().notify(&msg);

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
}
