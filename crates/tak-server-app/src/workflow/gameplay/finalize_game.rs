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
    moderation::Moderation,
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
            .try_then(move |match_data| {
                tracing::info!("Finalizing game {} in match {}", game.game_id, match_id);

                let mut match_data = match match_data {
                    Some(m) => m,
                    None => {
                        tracing::error!("Match {} not found", match_id);
                        return Err(TransactionError::new("Match not found"));
                    }
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
                Ok(match_repo
                    .update_match(match_id, match_data.clone())
                    .map_value(move |()| match_data.clone()))
            })
            .try_then(move |match_data| {
                if let MatchStatus::Completed = match_data.status {
                    if let Some(item) = tournament_match_workflow.handle_completed_match(match_data)
                    {
                        Ok(item)
                    } else {
                        Ok(TransactionItem::constant(Ok(())))
                    }
                } else {
                    Ok(TransactionItem::constant(Ok(())))
                }
            })
    }

    async fn update_ratings(
        &self,
        ended_game: &FinishedGame,
        white_account: Option<&Moderation>,
        black_account: Option<&Moderation>,
    ) -> Option<GameRatingInfo> {
        if white_account.is_none_or(|x| x.is_guest()) || black_account.is_none_or(|x| x.is_guest())
        {
            return None;
        };
        let white_id = ended_game.metadata.white_id;
        let black_id = ended_game.metadata.black_id;
        let ended_game_clone = ended_game.clone();
        let rating_service = self.rating_service.clone();
        let (white_rating_if_changed, black_rating_if_changed, info) = match self
            .ports
            .rating_repository()
            .update_player_ratings(
                ended_game.metadata.white_id,
                ended_game.metadata.black_id,
                move |w_rating, b_rating| {
                    let prev_white_rating = w_rating.as_ref().map(|r| r.rating);
                    let prev_black_rating = b_rating.as_ref().map(|r| r.rating);

                    let mut w_rating = w_rating.unwrap_or(PlayerRating::new(white_id));
                    let mut b_rating = b_rating.unwrap_or(PlayerRating::new(black_id));
                    let res = rating_service.calculate_ratings(
                        &ended_game_clone,
                        &mut w_rating,
                        &mut b_rating,
                    );
                    let white_rating_if_changed =
                        Some(w_rating.rating).filter(|&r| Some(r) != prev_white_rating);
                    let black_rating_if_changed =
                        Some(b_rating.rating).filter(|&r| Some(r) != prev_black_rating);
                    let info = res.map(|info| info);
                    (
                        w_rating,
                        b_rating,
                        (white_rating_if_changed, black_rating_if_changed, info),
                    )
                },
            )
            .await
        {
            Ok(res) => res,
            Err(e) => {
                tracing::error!(
                    "Failed to update player ratings for game {}: {}",
                    ended_game.game_id,
                    e
                );
                (None, None, None)
            }
        };
        if let Some(new_white_rating) = &white_rating_if_changed {
            if let Err(e) = self
                .ports
                .rating_history_repository()
                .add_rating_history_entry(
                    white_id,
                    RatingHistoryEntry::new(ended_game.metadata.date, *new_white_rating),
                )
                .await
            {
                tracing::error!(
                    "Failed to add rating history entry for player {}: {}",
                    white_id,
                    e
                );
            }
            tracing::debug!(
                "White player's rating changed to {} after game {}, history entry added",
                new_white_rating,
                ended_game.game_id
            );
        } else {
            tracing::debug!(
                "White player's rating did not change after game {}, no history entry added",
                ended_game.game_id
            );
        }
        if let Some(new_black_rating) = &black_rating_if_changed {
            if let Err(e) = self
                .ports
                .rating_history_repository()
                .add_rating_history_entry(
                    black_id,
                    RatingHistoryEntry::new(ended_game.metadata.date, *new_black_rating),
                )
                .await
            {
                tracing::error!(
                    "Failed to add rating history entry for player {}: {}",
                    black_id,
                    e
                );
            }
            tracing::debug!(
                "Black player's rating changed to {} after game {}, history entry added",
                new_black_rating,
                ended_game.game_id
            );
        } else {
            tracing::debug!(
                "Black player's rating did not change after game {}, no history entry added",
                ended_game.game_id
            );
        }
        info
    }

    async fn update_stats(
        &self,
        ended_game: &FinishedGame,
        white_account: Option<&Moderation>,
        black_account: Option<&Moderation>,
    ) {
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
            TakGameResult::Aborted => return,
        };

        let is_rated = ended_game.metadata.is_rated;

        if white_account.is_some_and(|x| !x.is_guest()) {
            if let Err(e) = self
                .ports
                .stats_repository()
                .update_player_game(ended_game.metadata.white_id, move |stats| {
                    Self::update_stats_fn(stats, white_outcome, is_rated)
                })
                .await
            {
                tracing::error!(
                    "Failed to update stats for player {}: {}",
                    ended_game.metadata.white_id,
                    e
                );
            }
        } else {
            tracing::debug!(
                "White player {} is a guest, not updating stats",
                ended_game.metadata.white_id
            );
        }

        if black_account.is_some_and(|x| !x.is_guest()) {
            if let Err(e) = self
                .ports
                .stats_repository()
                .update_player_game(ended_game.metadata.black_id, move |stats| {
                    Self::update_stats_fn(stats, black_outcome, is_rated)
                })
                .await
            {
                tracing::error!(
                    "Failed to update stats for player {}: {}",
                    ended_game.metadata.black_id,
                    e
                );
            }
        } else {
            tracing::debug!(
                "Black player {} is a guest, not updating stats",
                ended_game.metadata.black_id
            );
        }
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
            .moderation_repository()
            .get_moderation(ended_game.metadata.white_id)
            .await
        {
            Ok(account) => Some(account),
            Err(e) => {
                tracing::error!(
                    "Failed to retrieve account for white player {}: {:?}",
                    ended_game.metadata.white_id,
                    e
                );
                None
            }
        };
        let black_account = match self
            .ports
            .moderation_repository()
            .get_moderation(ended_game.metadata.black_id)
            .await
        {
            Ok(account) => Some(account),
            Err(e) => {
                tracing::error!(
                    "Failed to retrieve account for black player {}: {:?}",
                    ended_game.metadata.black_id,
                    e
                );
                None
            }
        };

        let game_rating_info = self
            .update_ratings(&ended_game, white_account.as_ref(), black_account.as_ref())
            .await;

        tracing::debug!(
            "Ratings updated for game {}, updating match and game history",
            game_id
        );

        if let Some(match_id) = ended_game.metadata.match_id {
            self.ports
                .transaction_manager()
                .execute_without_transaction(self.handle_match(match_id, ended_game.clone()))
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

        self.update_stats(&ended_game, white_account.as_ref(), black_account.as_ref())
            .await;

        if let Err(e) = self
            .ports
            .game_repository()
            .update_finished_game(
                game_id,
                ended_game.game.game_result().clone(),
                ended_game.events.clone(),
                game_rating_info,
            )
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
