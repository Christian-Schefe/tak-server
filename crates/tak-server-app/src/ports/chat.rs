use tak_domain::{
    ChatMessageId,
    chat::{ChatConversation, ChatMessage},
};

use crate::errors::{AppResult, InternalError};

#[async_trait::async_trait]
pub trait ChatRepository<RpCtx> {
    async fn save_message(
        &self,
        ctx: &RpCtx,
        conversation: ChatConversation,
        message: ChatMessage,
    ) -> AppResult<ChatMessageId, (InternalError,)>;
    async fn get_messages(
        &self,
        ctx: &RpCtx,
        conversation: ChatConversation,
        cursor: Option<ChatMessageId>,
        limit: usize,
    ) -> AppResult<Vec<(ChatMessageId, ChatMessage)>, (InternalError,)>;
}
