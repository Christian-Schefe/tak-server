use rustrict::{Censor, Type};
use unordered_pair::UnorderedPair;

use crate::UserId;

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub date: chrono::DateTime<chrono::Utc>,
    pub sender: UserId,
    pub message: String,
}

#[derive(Clone, Debug)]
pub enum ChatConversation {
    Private { user_ids: UnorderedPair<UserId> },
    Room { room_name: String },
    Global,
}

pub trait ContentPolicy {
    fn filter_message(&self, message: &str) -> Result<String, String>;
}

pub struct RustrictContentPolicy;

impl RustrictContentPolicy {
    const MAX_MESSAGE_LENGTH: usize = 2000;
}

impl ContentPolicy for RustrictContentPolicy {
    fn filter_message(&self, message: &str) -> Result<String, String> {
        if message.is_empty() {
            return Err("Message cannot be empty".to_string());
        }
        if message.len() > Self::MAX_MESSAGE_LENGTH {
            return Err(format!(
                "Message cannot be longer than {} characters",
                Self::MAX_MESSAGE_LENGTH
            ));
        }

        let (censored, censor_type) = Censor::from_str(message)
            .with_censor_threshold(Type::INAPPROPRIATE)
            .censor_and_analyze();
        if censor_type.is(Type::INAPPROPRIATE) {
            Ok(censored)
        } else {
            Ok(message.to_string())
        }
    }
}
