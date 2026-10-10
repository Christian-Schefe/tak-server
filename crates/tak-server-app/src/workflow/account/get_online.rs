use std::sync::Arc;

use crate::{domain::UserId, ports::connection::AccountOnlineStatusPort};

pub trait GetOnlineAccountsUseCase {
    fn get_online_accounts(&self) -> Vec<UserId>;
}

pub struct GetOnlineAccountsUseCaseImpl<P: AccountOnlineStatusPort> {
    account_online_status_port: Arc<P>,
}

impl<P> GetOnlineAccountsUseCaseImpl<P>
where
    P: AccountOnlineStatusPort + Send + Sync + 'static,
{
    pub fn new(account_online_status_port: Arc<P>) -> Self {
        Self {
            account_online_status_port,
        }
    }
}

impl<P> GetOnlineAccountsUseCase for GetOnlineAccountsUseCaseImpl<P>
where
    P: AccountOnlineStatusPort + Send + Sync + 'static,
{
    fn get_online_accounts(&self) -> Vec<UserId> {
        self.account_online_status_port.get_online_accounts()
    }
}
