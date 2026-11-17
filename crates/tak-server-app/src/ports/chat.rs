use std::sync::Arc;

use more_concurrent_maps::multi::ConcurrentMultiMap;
use tak_domain::{
    ChatMessageId, ListenerId,
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
