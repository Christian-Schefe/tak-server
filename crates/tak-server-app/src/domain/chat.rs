use std::sync::Arc;

use more_concurrent_maps::multi::ConcurrentMultiMap;
use rustrict::{Censor, Type};
use unordered_pair::UnorderedPair;

use crate::domain::{AccountId, ChatMessageId, ListenerId, RepoError};

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

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub date: chrono::DateTime<chrono::Utc>,
    pub sender: AccountId,
    pub message: String,
}

#[derive(Clone, Debug)]
pub enum ChatConversation {
    Private {
        account_ids: UnorderedPair<AccountId>,
    },
    Room {
        room_name: String,
    },
    Global,
}

pub trait ChatRoomService {
    fn join_room(&self, room_name: &String, listener_id: ListenerId);
    fn leave_room(&self, room_name: &String, listener_id: ListenerId);
    fn leave_all_rooms(&self, listener_id: ListenerId);
    fn get_listeners_in_room(&self, room_name: &String) -> Vec<ListenerId>;
}

pub struct ChatRoomServiceImpl {
    rooms: Arc<ConcurrentMultiMap<String, ListenerId>>,
}

impl ChatRoomServiceImpl {
    pub fn new() -> Self {
        Self {
            rooms: Arc::new(ConcurrentMultiMap::new()),
        }
    }
}

impl ChatRoomService for ChatRoomServiceImpl {
    fn join_room(&self, room_name: &String, listener_id: ListenerId) {
        self.rooms.insert(room_name.to_string(), listener_id);
    }

    fn leave_room(&self, room_name: &String, listener_id: ListenerId) {
        self.rooms.remove(room_name, &listener_id);
    }

    fn leave_all_rooms(&self, listener_id: ListenerId) {
        self.rooms.remove_by_right(&listener_id);
    }

    fn get_listeners_in_room(&self, room_name: &String) -> Vec<ListenerId> {
        self.rooms.get_by_left(room_name)
    }
}

pub trait ContentPolicy {
    fn filter_message(&self, message: &str) -> Result<String, String>;
}

pub struct RustrictContentPolicy;

impl RustrictContentPolicy {
    const MAX_MESSAGE_LENGTH: usize = 2000;
    pub fn new() -> Self {
        Self {}
    }
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
