use std::sync::Arc;

use tak_core::{TakGameResult, TakPlayer};

use crate::{
    domain::{
        MatchId,
        game::FinishedGame,
        game_history::{GameHistoryService, GameRatingInfo, GameRepository},
        matches::{MatchRepository, MatchStatus},
        rating::{PlayerRating, RatingRepository, RatingService},
        spectator::SpectatorService,
        stats::{
            GameOutcome, PlayerStats, RatingHistoryEntry, RatingHistoryRepository, StatsRepository,
        },
    },
    ports::notification::{ListenerGameMessageType, ListenerMessage, ListenerNotificationPort},
    workflow::{
        account::get_account::GetAccountWorkflow, gameplay::FinishedGameView,
        player::notify_player::NotifyPlayerWorkflow,
        tournament::tournament_match::TournamentMatchWorkflow,
    },
};

#[async_trait::async_trait]
pub trait FinalizeGameWorkflow {
    async fn finalize_game(&self, ended_game: FinishedGame);
}

pub struct FinalizeGameWorkflowImpl<
    G: GameRepository,
    R: RatingService,
    RP: RatingRepository,
    GH: GameHistoryService,
    M: MatchRepository,
    NP: NotifyPlayerWorkflow,
    SPS: SpectatorService,
    L: ListenerNotificationPort,
    A: GetAccountWorkflow,
    S: StatsRepository,
    RH: RatingHistoryRepository,
    TM: TournamentMatchWorkflow,
> {
    game_repository: Arc<G>,
    rating_service: Arc<R>,
    rating_repository: Arc<RP>,
    game_history_service: Arc<GH>,
    match_repository: Arc<M>,
    notify_player_workflow: Arc<NP>,
    spectator_service: Arc<SPS>,
    listener_notification_port: Arc<L>,
    get_account_workflow: Arc<A>,
    stats_repository: Arc<S>,
    rating_history_repository: Arc<RH>,
    tournament_match_workflow: Arc<TM>,
}

impl<
    G: GameRepository,
    R: RatingService,
    RP: RatingRepository,
    GH: GameHistoryService,
    M: MatchRepository,
    NP: NotifyPlayerWorkflow,
    SPS: SpectatorService,
    L: ListenerNotificationPort,
    A: GetAccountWorkflow,
    S: StatsRepository,
    RH: RatingHistoryRepository,
    TM: TournamentMatchWorkflow,
> FinalizeGameWorkflowImpl<G, R, RP, GH, M, NP, SPS, L, A, S, RH, TM>
{
    pub fn new(
        game_repository: Arc<G>,
        rating_service: Arc<R>,
        rating_repository: Arc<RP>,
        game_history_service: Arc<GH>,
        match_repository: Arc<M>,
        notify_player_workflow: Arc<NP>,
        spectator_service: Arc<SPS>,
        listener_notification_port: Arc<L>,
        get_account_workflow: Arc<A>,
        stats_repository: Arc<S>,
        rating_history_repository: Arc<RH>,
        tournament_match_workflow: Arc<TM>,
    ) -> Self {
        Self {
            game_repository,
            rating_service,
            rating_repository,
            game_history_service,
            match_repository,
            notify_player_workflow,
            spectator_service,
            listener_notification_port,
            get_account_workflow,
            stats_repository,
            rating_history_repository,
            tournament_match_workflow,
        }
    }

    async fn handle_match(&self, match_id: MatchId, game: &FinishedGame) {
        tracing::info!("Finalizing game {} in match {}", game.game_id, match_id);
        let winner = match game.game.game_result() {
            TakGameResult::Draw => None,
            TakGameResult::Win {
                winner: TakPlayer::White,
                ..
            } => Some(game.metadata.white_id),
            TakGameResult::Win {
                winner: TakPlayer::Black,
                ..
            } => Some(game.metadata.black_id),
        };
        let mut match_data = match self.match_repository.get_match(match_id).await {
            Ok(m) => m,
            Err(e) => {
                tracing::error!("Failed to retrieve match {}: {}", match_id, e);
                return;
            }
        };
        match_data.end_game_in_match(winner);

        if let Err(e) = self
            .match_repository
            .update_match(match_id, match_data.clone())
            .await
        {
            tracing::error!("Failed to update match {}: {}", match_id, e);
        } else {
            tracing::info!(
                "Match {} updated successfully after game {}",
                match_id,
                game.game_id
            );
        }

        if let MatchStatus::Completed = match_data.status {
            self.tournament_match_workflow
                .handle_completed_match(match_data)
                .await;
        }
    }
}

#[async_trait::async_trait]
impl<
    G: GameRepository + Send + Sync + 'static,
    R: RatingService + Send + Sync + 'static,
    RP: RatingRepository + Send + Sync + 'static,
    GH: GameHistoryService + Send + Sync + 'static,
    M: MatchRepository + Send + Sync + 'static,
    NP: NotifyPlayerWorkflow + Send + Sync + 'static,
    SPS: SpectatorService + Send + Sync + 'static,
    L: ListenerNotificationPort + Send + Sync + 'static,
    A: GetAccountWorkflow + Send + Sync + 'static,
    S: StatsRepository + Send + Sync + 'static,
    RH: RatingHistoryRepository + Send + Sync + 'static,
    TM: TournamentMatchWorkflow + Send + Sync + 'static,
> FinalizeGameWorkflow for FinalizeGameWorkflowImpl<G, R, RP, GH, M, NP, SPS, L, A, S, RH, TM>
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

        let game_rating_info = update_ratings(
            &self.get_account_workflow,
            &self.rating_service,
            &self.rating_repository,
            &self.rating_history_repository,
            &ended_game,
        )
        .await;

        tracing::debug!(
            "Ratings updated for game {}, updating match and game history",
            game_id
        );

        if let Some(match_id) = ended_game.metadata.match_id {
            self.handle_match(match_id, &ended_game).await;
        } else {
            tracing::info!("Game {} is not part of a match", game_id);
        }

        update_stats(&self.stats_repository, &ended_game).await;

        let game_record_update = self
            .game_history_service
            .get_finished_game_record_update(ended_game, game_rating_info);
        if let Err(e) = self
            .game_repository
            .update_finished_game(game_id, game_record_update)
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

async fn update_ratings<
    A: GetAccountWorkflow,
    RS: RatingService + Send + Sync + 'static,
    RR: RatingRepository,
    RH: RatingHistoryRepository,
>(
    get_account_workflow: &Arc<A>,
    rating_service: &Arc<RS>,
    rating_repository: &Arc<RR>,
    rating_history_repository: &Arc<RH>,
    ended_game: &FinishedGame,
) -> Option<GameRatingInfo> {
    let white_account = get_account_workflow
        .get_account(ended_game.metadata.white_id)
        .await
        .ok();
    let black_account = get_account_workflow
        .get_account(ended_game.metadata.black_id)
        .await
        .ok();

    if white_account.is_none_or(|x| x.is_guest()) || black_account.is_none_or(|x| x.is_guest()) {
        None
    } else {
        let white_id = ended_game.metadata.white_id;
        let black_id = ended_game.metadata.black_id;
        let ended_game_clone = ended_game.clone();
        let rating_service = rating_service.clone();
        let (white_rating_if_changed, black_rating_if_changed, info) = match rating_repository
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
            if let Err(e) = rating_history_repository
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
            if let Err(e) = rating_history_repository
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
}

async fn update_stats<S: StatsRepository>(stats_repository: &Arc<S>, ended_game: &FinishedGame) {
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
    };

    let is_rated = ended_game.metadata.is_rated;
    if let Err(e) = stats_repository
        .update_player_game(ended_game.metadata.white_id, move |stats| {
            update_stats_fn(stats, white_outcome, is_rated)
        })
        .await
    {
        tracing::error!(
            "Failed to update stats for player {}: {}",
            ended_game.metadata.white_id,
            e
        );
    }

    if let Err(e) = stats_repository
        .update_player_game(ended_game.metadata.black_id, move |stats| {
            update_stats_fn(stats, black_outcome, is_rated)
        })
        .await
    {
        tracing::error!(
            "Failed to update stats for player {}: {}",
            ended_game.metadata.black_id,
            e
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
