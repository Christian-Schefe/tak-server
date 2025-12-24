use std::sync::Arc;

use more_dashmap::many_many::ManyManyDashMap;
use rustrict::CensorStr;

use crate::app::domain::ListenerId;

pub trait ChatRoomService {
    fn join_room(&self, room_name: &String, listener_id: ListenerId);
    fn leave_room(&self, room_name: &String, listener_id: ListenerId);
    fn leave_all_rooms(&self, listener_id: ListenerId);
    fn get_listeners_in_room(&self, room_name: &String) -> Vec<ListenerId>;
}

pub struct ChatRoomServiceImpl {
    rooms: Arc<ManyManyDashMap<String, ListenerId>>,
}

impl ChatRoomServiceImpl {
    pub fn new() -> Self {
        Self {
            rooms: Arc::new(ManyManyDashMap::new()),
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
        self.rooms.remove_value(&listener_id);
    }

    fn get_listeners_in_room(&self, room_name: &String) -> Vec<ListenerId> {
        self.rooms.get_by_key(room_name)
    }
}

pub trait ContentPolicy {
    fn filter_message(&self, message: &str) -> String;
}

pub struct RustrictContentPolicy;

impl RustrictContentPolicy {
    pub fn new() -> Self {
        Self {}
    }
}

impl ContentPolicy for RustrictContentPolicy {
    fn filter_message(&self, message: &str) -> String {
        message.censor()
    }
}
