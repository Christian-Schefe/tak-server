use std::sync::Arc;

use more_concurrent_maps::multi::ConcurrentMultiMap;
use tak_domain::{
    ChatMessageId, UserId,
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

pub trait ChatRoomService {
    fn join_room(&self, user_id: UserId, room_name: &String);
    fn leave_room(&self, user_id: UserId, room_name: &String);
    fn leave_all_rooms(&self, user_id: UserId);
    fn get_users_in_room(&self, room_name: &String) -> Vec<UserId>;
}

pub struct ChatRoomServiceImpl {
    rooms: Arc<ConcurrentMultiMap<String, UserId>>,
}

impl ChatRoomServiceImpl {
    pub fn new() -> Self {
        Self {
            rooms: Arc::new(ConcurrentMultiMap::new()),
        }
    }
}

impl ChatRoomService for ChatRoomServiceImpl {
    fn join_room(&self, user_id: UserId, room_name: &String) {
        self.rooms.insert(room_name.to_string(), user_id);
    }

    fn leave_room(&self, user_id: UserId, room_name: &String) {
        self.rooms.remove(room_name, &user_id);
    }

    fn leave_all_rooms(&self, user_id: UserId) {
        self.rooms.remove_by_right(&user_id);
    }

    fn get_users_in_room(&self, room_name: &String) -> Vec<UserId> {
        self.rooms.get_by_left(room_name)
    }
}
