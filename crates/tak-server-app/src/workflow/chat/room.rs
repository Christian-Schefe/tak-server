use std::sync::Arc;

use crate::domain::{ListenerId, chat::ChatRoomService};

pub trait ChatRoomUseCase {
    fn join_room(&self, room_name: &String, listener_id: ListenerId);
    fn leave_room(&self, room_name: &String, listener_id: ListenerId);
}

pub struct ChatRoomUseCaseImpl<R: ChatRoomService> {
    chat_room_service: Arc<R>,
}

impl<R: ChatRoomService> ChatRoomUseCaseImpl<R> {
    pub fn new(chat_room_service: Arc<R>) -> Self {
        Self { chat_room_service }
    }
}

impl<R: ChatRoomService> ChatRoomUseCase for ChatRoomUseCaseImpl<R> {
    fn join_room(&self, room_name: &String, listener_id: ListenerId) {
        self.chat_room_service.join_room(room_name, listener_id);
    }

    fn leave_room(&self, room_name: &String, listener_id: ListenerId) {
        self.chat_room_service.leave_room(room_name, listener_id);
    }
}
