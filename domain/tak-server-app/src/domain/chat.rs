use std::sync::Arc;

use more_dashmap::many_many::ManyManyDashMap;
use rustrict::CensorStr;

use crate::domain::PlayerId;

pub trait ChatRoomService {
    fn add_player_to_room(&self, room_name: String, player_id: PlayerId);
    fn remove_player_from_room(&self, room_name: String, player_id: PlayerId);
    fn remove_player_from_all_rooms(&self, player_id: PlayerId);
    fn get_players_in_room(&self, room_name: &String) -> Vec<PlayerId>;
}

pub struct ChatRoomServiceImpl {
    rooms: Arc<ManyManyDashMap<String, PlayerId>>,
}

impl ChatRoomServiceImpl {
    pub fn new() -> Self {
        Self {
            rooms: Arc::new(ManyManyDashMap::new()),
        }
    }
}

impl ChatRoomService for ChatRoomServiceImpl {
    fn add_player_to_room(&self, room_name: String, player_id: PlayerId) {
        self.rooms.insert(room_name, player_id);
    }

    fn remove_player_from_room(&self, room_name: String, player_id: PlayerId) {
        self.rooms.remove(&room_name, &player_id);
    }

    fn remove_player_from_all_rooms(&self, player_id: PlayerId) {
        self.rooms.remove_value(&player_id);
    }

    fn get_players_in_room(&self, room_name: &String) -> Vec<PlayerId> {
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
