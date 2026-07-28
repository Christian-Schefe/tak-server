use std::sync::Arc;

use tak_core::TakTimeSettings;

use crate::{
    domain::{
        AccountId,
        matches::{MatchReadinessService, MatchRepository},
        seek::SeekService,
    },
    ports::{
        connection::AccountOnlineStatusPort,
        notification::{ListenerMatchEventType, ListenerMessage, ListenerNotificationPort},
    },
    processes::disconnect_timeout_runner::DisconnectTimeoutRunner,
    services::player_resolver::{PlayerResolverService, ResolveError},
    workflow::player::notify_player::NotifyPlayerWorkflow,
};

#[async_trait::async_trait]
pub trait SetAccountOnlineUseCase {
    async fn set_online(&self, account_id: &AccountId);
    async fn set_offline(&self, account_id: &AccountId);
}

pub struct SetAccountOnlineUseCaseImpl<
    P: AccountOnlineStatusPort,
    L: ListenerNotificationPort,
    S: SeekService,
    R: PlayerResolverService,
    D: DisconnectTimeoutRunner,
    M: MatchReadinessService,
    MR: MatchRepository,
    NP: NotifyPlayerWorkflow,
> {
    account_online_status_port: Arc<P>,
    notification_port: Arc<L>,
    seek_service: Arc<S>,
    player_resolver_service: Arc<R>,
    disconnect_timeout_runner: Arc<D>,
    match_readiness_service: Arc<M>,
    match_repository: Arc<MR>,
    notify_player_workflow: Arc<NP>,
}

impl<
    P: AccountOnlineStatusPort,
    L: ListenerNotificationPort,
    S: SeekService,
    R: PlayerResolverService,
    D: DisconnectTimeoutRunner,
    M: MatchReadinessService,
    MR: MatchRepository,
    NP: NotifyPlayerWorkflow,
> SetAccountOnlineUseCaseImpl<P, L, S, R, D, M, MR, NP>
{
    pub fn new(
        account_online_status_port: Arc<P>,
        notification_port: Arc<L>,
        seek_service: Arc<S>,
        player_resolver_service: Arc<R>,
        disconnect_timeout_runner: Arc<D>,
        match_readiness_service: Arc<M>,
        match_repository: Arc<MR>,
        notify_player_workflow: Arc<NP>,
    ) -> Self {
        Self {
            account_online_status_port,
            notification_port,
            seek_service,
            player_resolver_service,
            disconnect_timeout_runner,
            match_readiness_service,
            match_repository,
            notify_player_workflow,
        }
    }
}

#[async_trait::async_trait]
impl<
    P: AccountOnlineStatusPort + Send + Sync + 'static,
    L: ListenerNotificationPort + Send + Sync + 'static,
    S: SeekService + Send + Sync + 'static,
    R: PlayerResolverService + Send + Sync + 'static,
    D: DisconnectTimeoutRunner + Send + Sync + 'static,
    M: MatchReadinessService + Send + Sync + 'static,
    MR: MatchRepository + Send + Sync + 'static,
    NP: NotifyPlayerWorkflow + Send + Sync + 'static,
> SetAccountOnlineUseCase for SetAccountOnlineUseCaseImpl<P, L, S, R, D, M, MR, NP>
{
    #[tracing::instrument(skip(self))]
    async fn set_online(&self, account_id: &AccountId) {
        if let Some(accounts) = self
            .account_online_status_port
            .set_account_online(account_id)
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
                self.disconnect_timeout_runner.cancel_disconnect_timeout(id);
            }
            Err(ResolveError::Internal) => {
                tracing::error!(
                    "Failed to resolve player ID when setting account online: {}",
                    account_id
                )
            }
        };
    }

    #[tracing::instrument(skip(self))]
    async fn set_offline(&self, account_id: &AccountId) {
        if let Some(accounts) = self
            .account_online_status_port
            .set_account_offline(account_id)
        {
            let message = ListenerMessage::AccountsOnline { accounts };
            self.notification_port.notify_all(&message);
        }
        let player_id = match self
            .player_resolver_service
            .resolve_player_id_by_account_id(account_id)
            .await
        {
            Ok(id) => id,
            Err(ResolveError::Internal) => {
                tracing::error!(
                    "Failed to resolve player ID when setting account offline: {}",
                    account_id
                );
                return;
            }
        };
        let cancelled_seeks = self.seek_service.cancel_player_seeks(player_id, |seek| {
            matches!(
                seek.game_settings.time_settings,
                TakTimeSettings::Realtime(_)
            )
        });
        for cancelled_seek in cancelled_seeks {
            let message = ListenerMessage::SeekCancelled {
                seek: cancelled_seek.into(),
            };
            self.notification_port.notify_all(&message);
        }
        let cancelled_match_readiness = self
            .match_readiness_service
            .set_player_not_ready_everywhere(player_id);
        let match_futures =
            cancelled_match_readiness
                .into_iter()
                .map(|match_id| async move {
                    (match_id, self.match_repository.get_match(match_id).await)
                });
        let matches = futures::future::join_all(match_futures).await;
        for (match_id, match_entry) in matches {
            match match_entry {
                Ok(match_entry) => {
                    let message = ListenerMessage::MatchEvent {
                        match_id: match_id,
                        event_type: ListenerMatchEventType::MatchReadinessChanged {
                            player_id: None,
                        },
                    };
                    self.notify_player_workflow
                        .notify_players(
                            &[match_entry.player1.player_id, match_entry.player2.player_id],
                            &message,
                        )
                        .await;
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to retrieve match {match_id} entry when setting account offline: {:?}",
                        e
                    );
                }
            }
        }
        DisconnectTimeoutRunner::start_disconnect_timeout(
            self.disconnect_timeout_runner.clone(),
            player_id,
        );
    }
}
