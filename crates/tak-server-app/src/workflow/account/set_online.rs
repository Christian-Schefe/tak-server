use std::sync::Arc;

use crate::{
    PortsCollection,
    ports::{
        connection::UserOnlineStatusPort,
        matches::MatchRepository,
        notification::{ListenerMatchEventType, ListenerMessage, ListenerNotificationPort},
        transaction::{RepoError, TransactionManager},
    },
    processes::disconnect_timeout_runner::DisconnectTimeoutRunner,
    workflow::player::notify_player::NotifyPlayerWorkflow,
};
use tak_core::TakTimeSettings;
use tak_domain::{UserId, matches::MatchReadinessService, seek::SeekService};

#[async_trait::async_trait]
pub trait SetAccountOnlineUseCase {
    async fn set_online(&self, user_id: UserId);
    async fn set_offline(&self, user_id: UserId);
}

pub struct SetAccountOnlineUseCaseImpl<
    Ports: PortsCollection,
    P: UserOnlineStatusPort,
    L: ListenerNotificationPort,
    S: SeekService,
    D: DisconnectTimeoutRunner,
    M: MatchReadinessService,
    NP: NotifyPlayerWorkflow,
> {
    ports: Arc<Ports>,
    account_online_status_port: Arc<P>,
    notification_port: Arc<L>,
    seek_service: Arc<S>,
    disconnect_timeout_runner: Arc<D>,
    match_readiness_service: Arc<M>,
    notify_player_workflow: Arc<NP>,
}

impl<
    Ports: PortsCollection,
    P: UserOnlineStatusPort,
    L: ListenerNotificationPort,
    S: SeekService,
    D: DisconnectTimeoutRunner,
    M: MatchReadinessService,
    NP: NotifyPlayerWorkflow,
> SetAccountOnlineUseCaseImpl<Ports, P, L, S, D, M, NP>
{
    pub fn new(
        ports: Arc<Ports>,
        account_online_status_port: Arc<P>,
        notification_port: Arc<L>,
        seek_service: Arc<S>,
        disconnect_timeout_runner: Arc<D>,
        match_readiness_service: Arc<M>,
        notify_player_workflow: Arc<NP>,
    ) -> Self {
        Self {
            ports,
            account_online_status_port,
            notification_port,
            seek_service,
            disconnect_timeout_runner,
            match_readiness_service,
            notify_player_workflow,
        }
    }
}

#[async_trait::async_trait]
impl<
    Ports: PortsCollection + Send + Sync + 'static,
    P: UserOnlineStatusPort + Send + Sync + 'static,
    L: ListenerNotificationPort + Send + Sync + 'static,
    S: SeekService + Send + Sync + 'static,
    D: DisconnectTimeoutRunner + Send + Sync + 'static,
    M: MatchReadinessService + Send + Sync + 'static,
    NP: NotifyPlayerWorkflow + Send + Sync + 'static,
> SetAccountOnlineUseCase for SetAccountOnlineUseCaseImpl<Ports, P, L, S, D, M, NP>
{
    #[tracing::instrument(skip(self))]
    async fn set_online(&self, user_id: UserId) {
        if let Some(users) = self.account_online_status_port.set_user_online(user_id) {
            let message = ListenerMessage::UsersOnline { users };
            self.notification_port.notify_all(&message);
        }
        self.disconnect_timeout_runner
            .cancel_disconnect_timeout(user_id);
    }

    #[tracing::instrument(skip(self))]
    async fn set_offline(&self, user_id: UserId) {
        if let Some(users) = self.account_online_status_port.set_user_offline(user_id) {
            let message = ListenerMessage::UsersOnline { users };
            self.notification_port.notify_all(&message);
        }
        let cancelled_seeks = self.seek_service.cancel_player_seeks(user_id, |seek| {
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
            .set_player_not_ready_everywhere(user_id);

        let ctx = self.ports.transaction_manager().simple();
        let ctx_ref = &ctx;
        let matches = match futures::future::join_all(cancelled_match_readiness.into_iter().map(
            |match_id| async move {
                self.ports
                    .match_repository()
                    .get_match(ctx_ref, match_id)
                    .await
                    .map(|opt_match| (match_id, opt_match))
            },
        ))
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        {
            Ok(matches) => matches,
            Err(RepoError(e)) => {
                tracing::error!(
                    "Failed to retrieve match entries when setting account offline: {:?}",
                    e
                );
                return;
            }
        };
        for (match_id, match_entry) in matches {
            match match_entry {
                Some(match_entry) => {
                    let message = ListenerMessage::MatchEvent {
                        match_id: match_id,
                        event_type: ListenerMatchEventType::MatchReadinessChanged { user_id: None },
                    };
                    self.notify_player_workflow
                        .notify_players(
                            &[match_entry.player1.user_id, match_entry.player2.user_id],
                            &message,
                        )
                        .await;
                }
                None => {
                    tracing::error!("Match {match_id} not found when setting account offline",);
                }
            }
        }
        DisconnectTimeoutRunner::start_disconnect_timeout(
            self.disconnect_timeout_runner.clone(),
            user_id,
        );
    }
}
