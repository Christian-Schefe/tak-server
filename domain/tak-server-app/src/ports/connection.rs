use crate::domain::{ListenerId, PlayerId};

pub trait PlayerConnectionPort {
    fn get_connection_id(&self, player_id: PlayerId) -> Option<ListenerId>;
}
