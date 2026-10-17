use tak_domain::{
    ChatMessageId,
    chat::{ChatConversation, ChatMessage},
};

use crate::ports::RepoError;

#[async_trait::async_trait]
pub trait ChatRepository {
    async fn save_message(
        &self,
        conversation: &ChatConversation,
        message: &ChatMessage,
    ) -> Result<ChatMessageId, RepoError>;
    async fn get_messages(
        &self,
        conversation: &ChatConversation,
        cursor: Option<ChatMessageId>,
        limit: usize,
    ) -> Result<Vec<(ChatMessageId, ChatMessage)>, RepoError>;
}
