use std::sync::Arc;

use chrono::{DateTime, Utc};

use better_errors::OneOf;
use tak_domain::{UserId, rating::PlayerRating, stats::PlayerStats};

use crate::{
    PortsCollection,
    application::matchmaking::SeekView,
    errors::{AppResult, InternalError, UserNotFound},
    ports::{
        PaginatedResponse,
        notification::{NotificationMatchEventType, NotificationMessage},
        rating::RatingQuery,
    },
    processes::disconnect_timeout_runner::DisconnectTimeoutRunner,
    workflow::get_snapshot::get_snapshot,
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

pub struct PlayerUseCaseImpl<Ports: PortsCollection, D: DisconnectTimeoutRunner> {
    ports: Arc<Ports>,
    disconnect_timeout_runner: Arc<D>,
}

impl<Ports: PortsCollection, D: DisconnectTimeoutRunner> PlayerUseCaseImpl<Ports, D> {
    pub fn new(ports: Arc<Ports>, disconnect_timeout_runner: Arc<D>) -> Self {
        Self {
            ports,
            disconnect_timeout_runner,
        }
    }
}

#[async_trait::async_trait]
impl<
    Ports: PortsCollection + Send + Sync + 'static,
    D: DisconnectTimeoutRunner + Send + Sync + 'static,
> PlayerUseCase for PlayerUseCaseImpl<Ports, D>
{
    async fn get_player(
        &self,
        user_id: UserId,
    ) -> AppResult<PlayerInfoView, (UserNotFound, InternalError)> {
        let ctx = self.ports.transaction_manager().simple();
        let snapshot = get_snapshot(self.ports.as_ref(), &ctx, user_id)
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
        self.ports.user_online_status_port().get_online_users()
    }
    #[tracing::instrument(skip(self))]
    async fn set_online(&self, user_id: UserId) {
        if let Some(users) = self
            .ports
            .user_online_status_port()
            .set_user_online(user_id)
        {
            let message = NotificationMessage::UsersOnline { users };
            self.ports.notification_port().notify(&message);
        }
        self.disconnect_timeout_runner
            .cancel_disconnect_timeout(user_id);
    }

    #[tracing::instrument(skip(self))]
    async fn set_offline(&self, user_id: UserId) -> AppResult<(), (InternalError,)> {
        if let Some(users) = self
            .ports
            .user_online_status_port()
            .set_user_offline(user_id)
        {
            let message = NotificationMessage::UsersOnline { users };
            self.ports.notification_port().notify(&message);
        }
        let cancelled_seeks = self.ports.seek_service().cancel_player_seeks(user_id, true);
        for (cancelled_seek_id, cancelled_seek) in cancelled_seeks {
            let message = NotificationMessage::SeekCancelled {
                seek: SeekView::from(cancelled_seek_id, &cancelled_seek),
            };
            self.ports.notification_port().notify(&message);
        }
        let cancelled_match_readiness = self
            .ports
            .match_readiness_service()
            .set_player_not_ready_everywhere(user_id);

        for match_id in cancelled_match_readiness {
            let message = NotificationMessage::MatchEvent {
                match_id: match_id,
                event_type: NotificationMatchEventType::MatchReadinessChanged { user_id: None },
            };
            self.ports.notification_port().notify(&message);
        }

        DisconnectTimeoutRunner::start_disconnect_timeout(
            self.disconnect_timeout_runner.clone(),
            user_id,
        );
        Ok(())
    }
}
