use tak_domain::{
    ChatMessageId,
    chat::{ChatConversation, ChatMessage},
};

use crate::ports::RepoError;

#[async_trait::async_trait]
pub trait ChatRepository<TrCtx> {
    async fn save_message(
        &self,
        ctx: &TrCtx,
        conversation: &ChatConversation,
        message: &ChatMessage,
    ) -> Result<ChatMessageId, RepoError>;
    async fn get_messages(
        &self,
        ctx: &TrCtx,
        conversation: &ChatConversation,
        cursor: Option<ChatMessageId>,
        limit: usize,
    ) -> Result<Vec<(ChatMessageId, ChatMessage)>, RepoError>;
}
