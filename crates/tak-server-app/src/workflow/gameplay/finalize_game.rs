use std::sync::Arc;

use crate::{
    PortsCollection,
    ports::{
        game_history::GameRepository,
        matches::MatchRepository,
        moderation::ModerationRepository,
        notification::{ListenerGameMessageType, ListenerMessage, ListenerNotificationPort},
        rating::RatingRepository,
        stats::{RatingHistoryRepository, StatsRepository},
        transaction::{TransactionError, TransactionItem, TransactionManager},
    },
    workflow::{
        gameplay::FinishedGameView, player::notify_player::NotifyPlayerWorkflow,
        tournament::tournament_match::TournamentMatchWorkflow,
    },
};
use tak_core::{TakGameResult, TakPlayer};
use tak_domain::{
    MatchId,
    game::FinishedGame,
    game_history::GameRatingInfo,
    matches::MatchStatus,
    rating::{PlayerRating, RatingService},
    spectator::SpectatorService,
    stats::{GameOutcome, PlayerStats, RatingHistoryEntry},
};

#[async_trait::async_trait]
pub trait FinalizeGameWorkflow {
    async fn finalize_game(&self, ended_game: FinishedGame);
}

pub struct FinalizeGameWorkflowImpl<
    Ports: PortsCollection,
    R: RatingService,
    NP: NotifyPlayerWorkflow,
    SPS: SpectatorService,
    L: ListenerNotificationPort,
    TM: TournamentMatchWorkflow<Ports::TrCtx>,
> {
    ports: Arc<Ports>,
    rating_service: Arc<R>,
    notify_player_workflow: Arc<NP>,
    spectator_service: Arc<SPS>,
    listener_notification_port: Arc<L>,
    tournament_match_workflow: Arc<TM>,
}

impl<
    Ports: PortsCollection,
    R: RatingService,
    NP: NotifyPlayerWorkflow,
    SPS: SpectatorService,
    L: ListenerNotificationPort,
    TM: TournamentMatchWorkflow<Ports::TrCtx>,
> FinalizeGameWorkflowImpl<Ports, R, NP, SPS, L, TM>
{
    pub fn new(
        ports: Arc<Ports>,
        rating_service: Arc<R>,
        notify_player_workflow: Arc<NP>,
        spectator_service: Arc<SPS>,
        listener_notification_port: Arc<L>,
        tournament_match_workflow: Arc<TM>,
    ) -> Self {
        Self {
            ports,
            rating_service,
            notify_player_workflow,
            spectator_service,
            listener_notification_port,
            tournament_match_workflow,
        }
    }
}
impl<
    Ports: PortsCollection + Send + Sync + 'static,
    R: RatingService + Send + Sync + 'static,
    NP: NotifyPlayerWorkflow + Send + Sync + 'static,
    SPS: SpectatorService + Send + Sync + 'static,
    L: ListenerNotificationPort + Send + Sync + 'static,
    TM: TournamentMatchWorkflow<Ports::TrCtx> + Send + Sync + 'static,
> FinalizeGameWorkflowImpl<Ports, R, NP, SPS, L, TM>
{
    fn handle_match(
        &self,
        match_id: MatchId,
        game: FinishedGame,
    ) -> TransactionItem<Ports::TrCtx, Result<(), TransactionError>> {
        let match_repo = self.ports.match_repository();
        let tournament_match_workflow = self.tournament_match_workflow.clone();
        match_repo
            .get_match(match_id)
            .flat_map_value(move |match_data| {
                let Some(mut match_data) = match_data else {
                    tracing::error!("Match {} not found", match_id);
                    return Err(TransactionError::new("Match not found"));
                };
                match game.game.game_result() {
                    TakGameResult::Draw => match_data.end_game_in_match(None),
                    TakGameResult::Win { winner, .. } => {
                        let winner_id = match winner {
                            TakPlayer::White => game.metadata.white_id,
                            TakPlayer::Black => game.metadata.black_id,
                        };
                        let is_player_one = if winner_id == match_data.player1.user_id {
                            true
                        } else if winner_id == match_data.player2.user_id {
                            false
                        } else {
                            tracing::error!(
                                "Winner ID {} does not match any player in the match",
                                winner_id
                            );
                            return Err(TransactionError::new(
                                "Winner ID does not match any player in the match",
                            ));
                        };
                        match_data.end_game_in_match(Some(is_player_one));
                    }
                    TakGameResult::Aborted => match_data.abort_game_in_match(),
                };
                tracing::info!("Finalizing game {} in match {}", game.game_id, match_id);
                Ok(match_data)
            })
            .try_then_ignore(move |match_data| {
                match_repo.update_match(match_id, match_data.clone())
            })
            .try_then(move |match_data| {
                if let MatchStatus::Completed = match_data.status
                    && let Some(item) = tournament_match_workflow.handle_completed_match(match_data)
                {
                    item
                } else {
                    TransactionItem::constant(Ok(()))
                }
            })
    }

    fn update_ratings(
        &self,
        ended_game: FinishedGame,
    ) -> TransactionItem<Ports::TrCtx, Result<Option<GameRatingInfo>, TransactionError>> {
        let white_id = ended_game.metadata.white_id;
        let black_id = ended_game.metadata.black_id;
        let rating_service = self.rating_service.clone();
        let rating_repo = self.ports.rating_repository();
        let rating_history_repo = self.ports.rating_history_repository();
        let date = ended_game.metadata.date;

        rating_repo
            .get_player_rating(white_id)
            .try_simultaneous(rating_repo.get_player_rating(black_id))
            .try_then(move |(w_rating, b_rating)| {
                let prev_white_rating = w_rating.as_ref().map(|r| r.rating);
                let prev_black_rating = b_rating.as_ref().map(|r| r.rating);

                let mut w_rating = w_rating.unwrap_or(PlayerRating::new(white_id));
                let mut b_rating = b_rating.unwrap_or(PlayerRating::new(black_id));
                let res =
                    rating_service.calculate_ratings(&ended_game, &mut w_rating, &mut b_rating);
                let white_rating_if_changed =
                    Some(w_rating.rating).filter(|&r| Some(r) != prev_white_rating);
                let black_rating_if_changed =
                    Some(b_rating.rating).filter(|&r| Some(r) != prev_black_rating);
                let info = res.map(|info| info);

                rating_repo
                    .set_player_rating(white_id, w_rating.clone())
                    .try_simultaneous(rating_repo.set_player_rating(black_id, b_rating.clone()))
                    .map_value(move |_| (white_rating_if_changed, black_rating_if_changed, info))
            })
            .try_then(
                move |(white_rating_if_changed, black_rating_if_changed, info)| {
                    let mut items = Vec::new();
                    if let Some(new_white_rating) = &white_rating_if_changed {
                        items.push(rating_history_repo.add_rating_history_entry(
                            white_id,
                            RatingHistoryEntry::new(date, *new_white_rating),
                        ))
                    }
                    if let Some(new_black_rating) = &black_rating_if_changed {
                        items.push(rating_history_repo.add_rating_history_entry(
                            black_id,
                            RatingHistoryEntry::new(date, *new_black_rating),
                        ))
                    }
                    TransactionItem::try_parallel(items).map_value(move |_| info)
                },
            )
    }

    fn update_stats(
        &self,
        ended_game: &FinishedGame,
        update_white: bool,
        update_black: bool,
    ) -> TransactionItem<Ports::TrCtx, Result<(), TransactionError>> {
        let (white_outcome, black_outcome) = match ended_game.game.game_result() {
            TakGameResult::Draw => (GameOutcome::Draw, GameOutcome::Draw),
            TakGameResult::Win {
                winner: TakPlayer::White,
                ..
            } => (GameOutcome::Win, GameOutcome::Loss),
            TakGameResult::Win {
                winner: TakPlayer::Black,
                ..
            } => (GameOutcome::Loss, GameOutcome::Win),
            TakGameResult::Aborted => return TransactionItem::constant(Ok(())),
        };

        let is_rated = ended_game.metadata.is_rated;
        let mut items = Vec::new();
        let white_id = ended_game.metadata.white_id;
        let black_id = ended_game.metadata.black_id;
        if update_white {
            let stats_repo = self.ports.stats_repository();
            items.push(
                stats_repo
                    .get_player_stats(white_id)
                    .try_then(move |stats| {
                        let updated = Self::update_stats_fn(stats, white_outcome, is_rated);
                        stats_repo.set_player_stats(white_id, updated)
                    }),
            )
        }
        if update_black {
            let stats_repo = self.ports.stats_repository();
            items.push(
                stats_repo
                    .get_player_stats(black_id)
                    .try_then(move |stats| {
                        let updated = Self::update_stats_fn(stats, black_outcome, is_rated);
                        stats_repo.set_player_stats(black_id, updated)
                    }),
            )
        }
        TransactionItem::try_parallel(items).map_value(|_| ())
    }

    fn update_stats_fn(
        stats: Option<PlayerStats>,
        outcome: GameOutcome,
        was_rated: bool,
    ) -> PlayerStats {
        let mut stats = stats.unwrap_or(PlayerStats {
            rated_games_played: 0,
            games_played: 0,
            games_won: 0,
            games_lost: 0,
            games_drawn: 0,
            win_streak: 0,
            longest_win_streak: 0,
        });
        stats.games_played += 1;
        if was_rated {
            stats.rated_games_played += 1;
        }
        match outcome {
            GameOutcome::Win => {
                stats.games_won += 1;
                stats.win_streak += 1;
                if stats.win_streak > stats.longest_win_streak {
                    stats.longest_win_streak = stats.win_streak;
                }
            }
            GameOutcome::Loss => {
                stats.games_lost += 1;
                stats.win_streak = 0;
            }
            GameOutcome::Draw => {
                stats.games_drawn += 1;
                stats.win_streak = 0;
            }
        }
        stats
    }
}

#[async_trait::async_trait]
impl<
    Ports: PortsCollection + Send + Sync + 'static,
    R: RatingService + Send + Sync + 'static,
    NP: NotifyPlayerWorkflow + Send + Sync + 'static,
    SPS: SpectatorService + Send + Sync + 'static,
    L: ListenerNotificationPort + Send + Sync + 'static,
    TM: TournamentMatchWorkflow<Ports::TrCtx> + Send + Sync + 'static,
> FinalizeGameWorkflow for FinalizeGameWorkflowImpl<Ports, R, NP, SPS, L, TM>
{
    #[tracing::instrument(skip(self, ended_game), fields(game_id = %ended_game.game_id))]
    async fn finalize_game(&self, ended_game: FinishedGame) {
        tracing::info!("Finalizing game {}", ended_game.game_id);
        let game_id = ended_game.game_id;
        let over_msg = ListenerMessage::GameEvent {
            game_id,
            event_type: ListenerGameMessageType::GameOver {
                game_result: ended_game.game.game_result().clone(),
            },
            time_info: ended_game.game.get_time_info(),
        };

        self.notify_player_workflow
            .notify_players(
                &[ended_game.metadata.white_id, ended_game.metadata.black_id],
                &over_msg,
            )
            .await;

        let observers = self.spectator_service.remove_game(game_id);
        self.listener_notification_port
            .notify_listeners(&observers, &over_msg);

        let ended_msg = ListenerMessage::GameEnded {
            game: FinishedGameView::from(&ended_game),
        };
        self.listener_notification_port.notify_all(&ended_msg);

        tracing::debug!(
            "Notified players and spectators about game {} ending, updating ratings and stats",
            game_id
        );

        let white_account = match self
            .ports
            .transaction_manager()
            .execute(
                self.ports
                    .moderation_repository()
                    .get_moderation(ended_game.metadata.white_id),
            )
            .await
        {
            Ok(account) => account,
            Err(TransactionError(e)) => {
                tracing::error!(
                    "Failed to retrieve account for white player {}: {}",
                    ended_game.metadata.white_id,
                    e
                );
                None
            }
        };
        let black_account = match self
            .ports
            .transaction_manager()
            .execute(
                self.ports
                    .moderation_repository()
                    .get_moderation(ended_game.metadata.black_id),
            )
            .await
        {
            Ok(account) => account,
            Err(TransactionError(e)) => {
                tracing::error!(
                    "Failed to retrieve account for black player {}: {}",
                    ended_game.metadata.black_id,
                    e
                );
                None
            }
        };
        let is_white_not_guest = white_account.as_ref().map_or(false, |x| !x.is_guest());
        let is_black_not_guest = black_account.as_ref().map_or(false, |x| !x.is_guest());

        let game_rating_info = if !(is_white_not_guest && is_black_not_guest) {
            None
        } else {
            match self
                .ports
                .transaction_manager()
                .execute(self.update_ratings(ended_game.clone()))
                .await
            {
                Ok(rating_info) => rating_info,
                Err(TransactionError(e)) => {
                    tracing::error!("Failed to update ratings for game {}: {}", game_id, e);
                    None
                }
            }
        };

        tracing::debug!(
            "Ratings updated for game {}, updating match and game history",
            game_id
        );

        if let Some(match_id) = ended_game.metadata.match_id {
            self.ports
                .transaction_manager()
                .execute(self.handle_match(match_id, ended_game.clone()))
                .await
                .unwrap_or_else(|TransactionError(e)| {
                    tracing::error!(
                        "Failed to update match {} for game {}: {:?}",
                        match_id,
                        game_id,
                        e
                    );
                });
        } else {
            tracing::info!("Game {} is not part of a match", game_id);
        }

        match self
            .ports
            .transaction_manager()
            .execute(self.update_stats(&ended_game, is_white_not_guest, is_black_not_guest))
            .await
        {
            Ok(_) => {
                tracing::debug!("Stats updated for game {}", game_id);
            }
            Err(TransactionError(e)) => {
                tracing::error!("Failed to update stats for game {}: {}", game_id, e);
            }
        }

        if let Err(TransactionError(e)) = self
            .ports
            .transaction_manager()
            .execute(self.ports.game_repository().update_finished_game(
                game_id,
                ended_game.game.game_result().clone(),
                ended_game.events.clone(),
                game_rating_info,
            ))
            .await
        {
            tracing::error!(
                "Failed to update finished game record for game {}: {}",
                game_id,
                e
            );
        }
        tracing::debug!("Finished finalizing game {}", game_id);
    }
}
