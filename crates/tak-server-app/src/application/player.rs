use std::sync::Arc;

use chrono::{DateTime, Utc};

use tak_core::TakTimeSettings;
use tak_domain::{UserId, rating::PlayerRating, stats::PlayerStats};
use terrors::OneOf;

use crate::{
    PortsCollection,
    application::matchmaking::SeekView,
    errors::{AppResult, InternalError, UserNotFound},
    ports::{
        PaginatedResponse,
        connection::UserOnlineStatusPort,
        matches::{MatchReadinessService, MatchRepository},
        notification::{ListenerMatchEventType, ListenerMessage, ListenerNotificationPort},
        rating::{RatingQuery, RatingRepository},
        seeks::SeekService,
        stats::{RatingHistoryRepository, StatsRepository},
        transaction::TransactionManager,
    },
    processes::disconnect_timeout_runner::DisconnectTimeoutRunner,
    workflow::{get_snapshot::GetSnapshotWorkflow, notify_player::NotifyPlayerWorkflow},
};

#[derive(Debug, Clone)]
pub struct PlayerInfoView {
    pub user_id: UserId,
    pub username: String,
    pub rating: Option<f64>,
}

pub struct PlayerStatsView {
    pub ranking: Option<PlayerRankingView>,
    pub rated_games_played: u32,
    pub games_played: u32,
    pub games_won: u32,
    pub games_lost: u32,
    pub games_drawn: u32,
    pub win_streak: u32,
    pub longest_win_streak: u32,
}

pub struct PlayerRankingView {
    pub rating: f64,
    pub max_rating: f64,
    pub ranking: u32,
}

impl PlayerStatsView {
    pub fn from(rating_info: Option<(u32, PlayerRating)>, stats: PlayerStats) -> Self {
        Self {
            ranking: rating_info.map(|(rank, rating)| PlayerRankingView {
                rating: rating.rating,
                max_rating: rating.max_rating,
                ranking: rank,
            }),
            rated_games_played: stats.rated_games_played,
            games_played: stats.games_played,
            games_won: stats.games_won,
            games_lost: stats.games_lost,
            games_drawn: stats.games_drawn,
            win_streak: stats.win_streak,
            longest_win_streak: stats.longest_win_streak,
        }
    }
}

pub struct RatingHistoryEntryView {
    pub timestamp: DateTime<Utc>,
    pub rating: f64,
}

pub struct RatingHistoryRangeView {
    pub entries: Vec<RatingHistoryEntryView>,
    pub first_entry_before_range: Option<RatingHistoryEntryView>,
}

#[async_trait::async_trait]
pub trait PlayerUseCase {
    async fn get_player(
        &self,
        user_id: UserId,
    ) -> AppResult<PlayerInfoView, (UserNotFound, InternalError)>;
    async fn get_stats(&self, user_id: UserId) -> AppResult<PlayerStatsView, (InternalError,)>;
    async fn get_rating_history(
        &self,
        user_id: UserId,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> AppResult<RatingHistoryRangeView, (InternalError,)>;
    async fn query_player_ratings(
        &self,
        query: RatingQuery,
    ) -> AppResult<PaginatedResponse<(UserId, f64)>, (InternalError,)>;
    fn get_online_users(&self) -> Vec<UserId>;
    async fn set_online(&self, user_id: UserId);
    async fn set_offline(&self, user_id: UserId) -> AppResult<(), (InternalError,)>;
}

pub struct PlayerUseCaseImpl<
    Ports: PortsCollection,
    P: UserOnlineStatusPort,
    L: ListenerNotificationPort,
    S: SeekService,
    D: DisconnectTimeoutRunner,
    M: MatchReadinessService,
    NP: NotifyPlayerWorkflow,
    SN: GetSnapshotWorkflow<Ports::TrCtx>,
> {
    ports: Arc<Ports>,
    user_online_status_port: Arc<P>,
    notification_port: Arc<L>,
    seek_service: Arc<S>,
    disconnect_timeout_runner: Arc<D>,
    match_readiness_service: Arc<M>,
    notify_player_workflow: Arc<NP>,
    get_snapshot_workflow: Arc<SN>,
}

impl<
    Ports: PortsCollection,
    P: UserOnlineStatusPort,
    L: ListenerNotificationPort,
    S: SeekService,
    D: DisconnectTimeoutRunner,
    M: MatchReadinessService,
    NP: NotifyPlayerWorkflow,
    SN: GetSnapshotWorkflow<Ports::TrCtx>,
> PlayerUseCaseImpl<Ports, P, L, S, D, M, NP, SN>
{
    pub fn new(
        ports: Arc<Ports>,
        user_online_status_port: Arc<P>,
        notification_port: Arc<L>,
        seek_service: Arc<S>,
        disconnect_timeout_runner: Arc<D>,
        match_readiness_service: Arc<M>,
        notify_player_workflow: Arc<NP>,
        get_snapshot_workflow: Arc<SN>,
    ) -> Self {
        Self {
            ports,
            user_online_status_port,
            notification_port,
            seek_service,
            disconnect_timeout_runner,
            match_readiness_service,
            notify_player_workflow,
            get_snapshot_workflow,
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
    SN: GetSnapshotWorkflow<Ports::TrCtx> + Send + Sync + 'static,
> PlayerUseCase for PlayerUseCaseImpl<Ports, P, L, S, D, M, NP, SN>
{
    async fn get_player(
        &self,
        user_id: UserId,
    ) -> AppResult<PlayerInfoView, (UserNotFound, InternalError)> {
        let ctx = self.ports.transaction_manager().simple();
        let snapshot = self
            .get_snapshot_workflow
            .get_snapshot(&ctx, user_id)
            .await
            .map_err(OneOf::broaden)?
            .ok_or(OneOf::new(UserNotFound))?;
        Ok(PlayerInfoView {
            user_id: snapshot.user_id,
            username: snapshot.username,
            rating: snapshot.rating,
        })
    }

    async fn get_stats(&self, user_id: UserId) -> AppResult<PlayerStatsView, (InternalError,)> {
        let ctx = self.ports.transaction_manager().simple();
        let rating_info = self
            .ports
            .rating_repository()
            .get_player_ranking(&ctx, user_id)
            .await?;
        let stats = self
            .ports
            .stats_repository()
            .get_player_stats(&ctx, user_id)
            .await?
            .unwrap_or_else(PlayerStats::new);
        Ok(PlayerStatsView::from(rating_info, stats))
    }

    async fn get_rating_history(
        &self,
        user_id: UserId,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> AppResult<RatingHistoryRangeView, (InternalError,)> {
        let ctx = self.ports.transaction_manager().simple();
        let rating_history_range = self
            .ports
            .rating_history_repository()
            .get_rating_history(&ctx, user_id, from, to)
            .await?;
        Ok(RatingHistoryRangeView {
            entries: rating_history_range
                .entries
                .into_iter()
                .map(|entry| RatingHistoryEntryView {
                    timestamp: entry.timestamp,
                    rating: entry.rating,
                })
                .collect(),
            first_entry_before_range: rating_history_range.first_entry_before_range.map(|entry| {
                RatingHistoryEntryView {
                    timestamp: entry.timestamp,
                    rating: entry.rating,
                }
            }),
        })
    }

    async fn query_player_ratings(
        &self,
        query: RatingQuery,
    ) -> AppResult<PaginatedResponse<(UserId, f64)>, (InternalError,)> {
        let ctx = self.ports.transaction_manager().simple();
        let paginated_ratings = self
            .ports
            .rating_repository()
            .query_player_ratings(&ctx, query)
            .await?;
        Ok(PaginatedResponse {
            total_count: paginated_ratings.total_count,
            items: paginated_ratings
                .items
                .into_iter()
                .map(|r| (r.user_id, r.rating))
                .collect(),
        })
    }

    fn get_online_users(&self) -> Vec<UserId> {
        self.user_online_status_port.get_online_users()
    }
    #[tracing::instrument(skip(self))]
    async fn set_online(&self, user_id: UserId) {
        if let Some(users) = self.user_online_status_port.set_user_online(user_id) {
            let message = ListenerMessage::UsersOnline { users };
            self.notification_port.notify_all(&message);
        }
        self.disconnect_timeout_runner
            .cancel_disconnect_timeout(user_id);
    }

    #[tracing::instrument(skip(self))]
    async fn set_offline(&self, user_id: UserId) -> AppResult<(), (InternalError,)> {
        if let Some(users) = self.user_online_status_port.set_user_offline(user_id) {
            let message = ListenerMessage::UsersOnline { users };
            self.notification_port.notify_all(&message);
        }
        let cancelled_seeks = self.seek_service.cancel_player_seeks(user_id, |seek| {
            matches!(
                seek.game_settings.time_settings,
                TakTimeSettings::Realtime(_)
            )
        });
        for (cancelled_seek_id, cancelled_seek) in cancelled_seeks {
            let message = ListenerMessage::SeekCancelled {
                seek: SeekView::from(cancelled_seek_id, &cancelled_seek),
            };
            self.notification_port.notify_all(&message);
        }
        let cancelled_match_readiness = self
            .match_readiness_service
            .set_player_not_ready_everywhere(user_id);

        let ctx = self.ports.transaction_manager().simple();
        let ctx_ref = &ctx;
        let matches = futures::future::join_all(cancelled_match_readiness.into_iter().map(
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
        .collect::<Result<Vec<_>, _>>()?;
        for (match_id, match_entry) in matches {
            match match_entry {
                Some(match_entry) => {
                    let message = ListenerMessage::MatchEvent {
                        match_id: match_id,
                        event_type: ListenerMatchEventType::MatchReadinessChanged { user_id: None },
                    };
                    self.notify_player_workflow.notify_players(
                        &[match_entry.player1.user_id, match_entry.player2.user_id],
                        &message,
                    );
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
        Ok(())
    }
}
