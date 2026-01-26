use std::sync::Arc;

use crate::{
    domain::{AccountId, seek::SeekService},
    ports::{
        connection::AccountOnlineStatusPort,
        notification::{ListenerMessage, ListenerNotificationPort},
    },
    services::player_resolver::{PlayerResolverService, ResolveError},
};

#[async_trait::async_trait]
pub trait SetAccountOnlineUseCase {
    fn set_online(&self, account_id: &AccountId);
    async fn set_offline(&self, account_id: &AccountId);
}

pub struct SetAccountOnlineUseCaseImpl<
    P: AccountOnlineStatusPort,
    L: ListenerNotificationPort,
    S: SeekService,
    R: PlayerResolverService,
> {
    account_online_status_port: Arc<P>,
    notification_port: Arc<L>,
    seek_service: Arc<S>,
    player_resolver_service: Arc<R>,
}

impl<
    P: AccountOnlineStatusPort,
    L: ListenerNotificationPort,
    S: SeekService,
    R: PlayerResolverService,
> SetAccountOnlineUseCaseImpl<P, L, S, R>
{
    pub fn new(
        account_online_status_port: Arc<P>,
        notification_port: Arc<L>,
        seek_service: Arc<S>,
        player_resolver_service: Arc<R>,
    ) -> Self {
        Self {
            account_online_status_port,
            notification_port,
            seek_service,
            player_resolver_service,
        }
    }
}

#[async_trait::async_trait]
impl<
    P: AccountOnlineStatusPort + Send + Sync + 'static,
    L: ListenerNotificationPort + Send + Sync + 'static,
    S: SeekService + Send + Sync + 'static,
    R: PlayerResolverService + Send + Sync + 'static,
> SetAccountOnlineUseCase for SetAccountOnlineUseCaseImpl<P, L, S, R>
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

    async fn set_offline(&self, account_id: &AccountId) {
        if let Some(accounts) = self
            .account_online_status_port
            .set_account_offline(account_id)
        {
            let message = ListenerMessage::AccountsOnline { accounts };
            self.notification_port.notify_all(&message);
        }
        match self
            .player_resolver_service
            .resolve_player_id_by_account_id(account_id)
            .await
        {
            Ok(id) => {
                self.seek_service.cancel_all_player_seeks(id);
            }
            Err(ResolveError::Internal) => {
                log::error!(
                    "Failed to resolve player ID when setting account offline: {}",
                    account_id
                )
            }
        };
    }
}
