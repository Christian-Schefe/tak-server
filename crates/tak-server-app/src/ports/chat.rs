use tak_domain::{
    ChatMessageId,
    chat::{ChatConversation, ChatMessage},
};

use crate::ports::transaction::RepoError;

#[async_trait::async_trait]
pub trait ChatRepository<RpCtx> {
    async fn save_message(
        &self,
        ctx: &RpCtx,
        conversation: ChatConversation,
        message: ChatMessage,
    ) -> Result<ChatMessageId, RepoError>;
    async fn get_messages(
        &self,
        ctx: &RpCtx,
        conversation: ChatConversation,
        cursor: Option<ChatMessageId>,
        limit: usize,
    ) -> Result<Vec<(ChatMessageId, ChatMessage)>, RepoError>;
}
