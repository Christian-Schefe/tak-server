use std::sync::Arc;

use crate::{
    ports::{
        connection::UserOnlineStatusPort,
        notification::{ListenerMatchEventType, ListenerMessage, ListenerNotificationPort},
    },
    processes::disconnect_timeout_runner::DisconnectTimeoutRunner,
    workflow::player::notify_player::NotifyPlayerWorkflow,
};
use tak_core::TakTimeSettings;
use tak_domain::{
    UserId,
    matches::{MatchReadinessService, MatchRepository},
    seek::SeekService,
};

#[async_trait::async_trait]
pub trait SetAccountOnlineUseCase {
    async fn set_online(&self, user_id: UserId);
    async fn set_offline(&self, user_id: UserId);
}

pub struct SetAccountOnlineUseCaseImpl<
    P: UserOnlineStatusPort,
    L: ListenerNotificationPort,
    S: SeekService,
    D: DisconnectTimeoutRunner,
    M: MatchReadinessService,
    MR: MatchRepository,
    NP: NotifyPlayerWorkflow,
> {
    account_online_status_port: Arc<P>,
    notification_port: Arc<L>,
    seek_service: Arc<S>,
    disconnect_timeout_runner: Arc<D>,
    match_readiness_service: Arc<M>,
    match_repository: Arc<MR>,
    notify_player_workflow: Arc<NP>,
}

impl<
    P: UserOnlineStatusPort,
    L: ListenerNotificationPort,
    S: SeekService,
    D: DisconnectTimeoutRunner,
    M: MatchReadinessService,
    MR: MatchRepository,
    NP: NotifyPlayerWorkflow,
> SetAccountOnlineUseCaseImpl<P, L, S, D, M, MR, NP>
{
    pub fn new(
        account_online_status_port: Arc<P>,
        notification_port: Arc<L>,
        seek_service: Arc<S>,
        disconnect_timeout_runner: Arc<D>,
        match_readiness_service: Arc<M>,
        match_repository: Arc<MR>,
        notify_player_workflow: Arc<NP>,
    ) -> Self {
        Self {
            account_online_status_port,
            notification_port,
            seek_service,
            disconnect_timeout_runner,
            match_readiness_service,
            match_repository,
            notify_player_workflow,
        }
    }
}

#[async_trait::async_trait]
impl<
    P: UserOnlineStatusPort + Send + Sync + 'static,
    L: ListenerNotificationPort + Send + Sync + 'static,
    S: SeekService + Send + Sync + 'static,
    D: DisconnectTimeoutRunner + Send + Sync + 'static,
    M: MatchReadinessService + Send + Sync + 'static,
    MR: MatchRepository + Send + Sync + 'static,
    NP: NotifyPlayerWorkflow + Send + Sync + 'static,
> SetAccountOnlineUseCase for SetAccountOnlineUseCaseImpl<P, L, S, D, M, MR, NP>
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
                        event_type: ListenerMatchEventType::MatchReadinessChanged { user_id: None },
                    };
                    self.notify_player_workflow
                        .notify_players(
                            &[match_entry.player1.user_id, match_entry.player2.user_id],
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
            user_id,
        );
    }
}
