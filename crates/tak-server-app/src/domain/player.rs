use crate::domain::UserId;

pub struct Player {
    pub user_id: UserId,
}

impl Player {
    pub fn new() -> Self {
        Self {
            user_id: UserId(uuid::Uuid::new_v4()),
        }
    }
}
