use std::sync::Arc;

use tak_domain::UserId;

use crate::ports::connection::UserOnlineStatusPort;

pub trait GetOnlineUsersUseCase {
    fn get_online_users(&self) -> Vec<UserId>;
}

pub struct GetOnlineUsersUseCaseImpl<P: UserOnlineStatusPort> {
    user_online_status_port: Arc<P>,
}

impl<P> GetOnlineUsersUseCaseImpl<P>
where
    P: UserOnlineStatusPort + Send + Sync + 'static,
{
    pub fn new(user_online_status_port: Arc<P>) -> Self {
        Self {
            user_online_status_port,
        }
    }
}

impl<P> GetOnlineUsersUseCase for GetOnlineUsersUseCaseImpl<P>
where
    P: UserOnlineStatusPort + Send + Sync + 'static,
{
    fn get_online_users(&self) -> Vec<UserId> {
        self.user_online_status_port.get_online_users()
    }
}
