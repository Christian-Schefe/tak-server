use tak_domain::{UserId, ChatMessageId, chat::ChatMessage};

pub mod message;
pub mod room;

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
