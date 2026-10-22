use tak_domain::{ListenerId, UserId};

pub trait UserConnectionPort {
    fn get_connection_id(&self, user_id: UserId) -> Option<ListenerId>;
}

pub trait UserOnlineStatusPort {
    fn set_user_online(&self, user_id: UserId) -> Option<Vec<UserId>>;
    fn set_user_offline(&self, user_id: UserId) -> Option<Vec<UserId>>;
    fn get_online_users(&self) -> Vec<UserId>;
}
