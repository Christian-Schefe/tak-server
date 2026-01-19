use std::sync::Arc;

use crate::{
    domain::AccountId,
    ports::{
        connection::AccountOnlineStatusPort,
        notification::{ListenerMessage, ListenerNotificationPort},
    },
};

pub trait SetAccountOnlineUseCase {
    fn set_online(&self, account_id: &AccountId);
    fn set_offline(&self, account_id: &AccountId);
}

pub struct SetAccountOnlineUseCaseImpl<P: AccountOnlineStatusPort, L: ListenerNotificationPort> {
    account_online_status_port: Arc<P>,
    notification_port: Arc<L>,
}

impl<P: AccountOnlineStatusPort, L: ListenerNotificationPort> SetAccountOnlineUseCaseImpl<P, L> {
    pub fn new(account_online_status_port: Arc<P>, notification_port: Arc<L>) -> Self {
        Self {
            account_online_status_port,
            notification_port,
        }
    }
}

impl<P: AccountOnlineStatusPort, L: ListenerNotificationPort> SetAccountOnlineUseCase
    for SetAccountOnlineUseCaseImpl<P, L>
{
    fn set_online(&self, account_id: &AccountId) {
        if let Some(accounts) = self
            .account_online_status_port
            .set_account_online(account_id)
        {
            let message = ListenerMessage::AccountsOnline { accounts };
            self.notification_port.notify_all(&message);
        }
    }

    fn set_offline(&self, account_id: &AccountId) {
        if let Some(accounts) = self
            .account_online_status_port
            .set_account_offline(account_id)
        {
            let message = ListenerMessage::AccountsOnline { accounts };
            self.notification_port.notify_all(&message);
        }
    }
}
