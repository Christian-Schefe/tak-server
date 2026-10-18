use tak_domain::{
    ChatMessageId,
    chat::{ChatConversation, ChatMessage},
};

use crate::ports::transaction::{TransactionError, TransactionItem};

pub trait ChatRepository<TrCtx> {
    fn save_message(
        &self,
        conversation: ChatConversation,
        message: ChatMessage,
    ) -> TransactionItem<TrCtx, Result<ChatMessageId, TransactionError>>;
    fn get_messages(
        &self,
        conversation: ChatConversation,
        cursor: Option<ChatMessageId>,
        limit: usize,
    ) -> TransactionItem<TrCtx, Result<Vec<(ChatMessageId, ChatMessage)>, TransactionError>>;
}
