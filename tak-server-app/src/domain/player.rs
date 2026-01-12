use crate::domain::PlayerId;

pub struct Player {
    pub player_id: PlayerId,
}

impl Player {
    pub fn new() -> Self {
        Self {
            player_id: PlayerId(uuid::Uuid::new_v4()),
        }
    }
}
