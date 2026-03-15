use std::sync::Arc;

use crate::{
    domain::{
        AccountId, ChatMessageId, RepoError,
        chat::{ChatConversation, ChatMessage, ChatRepository, ChatRoomService, ContentPolicy},
    },
    ports::{
        connection::AccountConnectionPort,
        notification::{ListenerMessage, ListenerNotificationPort},
    },
    workflow::chat::ChatMessageView,
};

#[async_trait::async_trait]
pub trait ChatMessageUseCase {
    async fn send_message(
        &self,
        from: &AccountId,
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
    L: ListenerNotificationPort,
    P: AccountConnectionPort,
    C: ChatRoomService,
    Co: ContentPolicy,
    CR: ChatRepository,
> {
    listener_notification_port: Arc<L>,
    player_connection_port: Arc<P>,
    chat_room_service: Arc<C>,
    content_policy: Arc<Co>,
    chat_repository: Arc<CR>,
}

impl<
    L: ListenerNotificationPort,
    P: AccountConnectionPort,
    C: ChatRoomService,
    Co: ContentPolicy,
    CR: ChatRepository,
> ChatMessageUseCaseImpl<L, P, C, Co, CR>
{
    pub fn new(
        listener_notification_port: Arc<L>,
        player_connection_port: Arc<P>,
        chat_room_service: Arc<C>,
        content_policy: Arc<Co>,
        chat_repository: Arc<CR>,
    ) -> Self {
        Self {
            listener_notification_port,
            player_connection_port,
            chat_room_service,
            content_policy,
            chat_repository,
        }
    }
}

#[async_trait::async_trait]
impl<
    L: ListenerNotificationPort + Send + Sync + 'static,
    P: AccountConnectionPort + Send + Sync + 'static,
    C: ChatRoomService + Send + Sync + 'static,
    Co: ContentPolicy + Send + Sync + 'static,
    CR: ChatRepository + Send + Sync + 'static,
> ChatMessageUseCase for ChatMessageUseCaseImpl<L, P, C, Co, CR>
{
    async fn send_message(
        &self,
        from_account_id: &AccountId,
        conversation: &ChatConversation,
        message: &str,
    ) -> Result<(), ChatSendMessageError> {
        let is_allowed_to_send = match conversation {
            ChatConversation::Private { account_ids } => {
                account_ids.0 == *from_account_id || account_ids.1 == *from_account_id
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

        let msg = ChatMessage {
            date: chrono::Utc::now(),
            sender: from_account_id.clone(),
            message: filtered_message.clone(),
        };

        if let Err(e) = self.chat_repository.save_message(conversation, &msg).await {
            log::error!("Failed to save chat message: {}", e);
            return Err(ChatSendMessageError::RepositoryError);
        }

        let msg = ListenerMessage::ChatMessage {
            from_account_id: from_account_id.clone(),
            message: filtered_message.clone(),
            conversation: conversation.clone(),
        };

        match conversation {
            ChatConversation::Private { account_ids } => {
                let member_listener_futures =
                    [&account_ids.0, &account_ids.1]
                        .into_iter()
                        .map(|to_account_id| async move {
                            self.player_connection_port
                                .get_connection_id(to_account_id)
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
        match self
            .chat_repository
            .get_messages(&conversation, before, limit)
            .await
        {
            Ok(messages) => Ok(messages
                .into_iter()
                .map(|(id, msg)| ChatMessageView::from(id, msg))
                .collect()),
            Err(RepoError::StorageError(e)) => {
                log::error!("Failed to retrieve chat messages: {}", e);
                Err(())
            }
        }
    }
}
