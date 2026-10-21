use std::sync::Arc;

use crate::{
    PortsCollection,
    ports::{
        game::GameRepository,
        matches::MatchRepository,
        moderation::ModerationRepository,
        notification::{ListenerGameMessageType, ListenerMessage, ListenerNotificationPort},
        rating::RatingRepository,
        stats::{RatingHistoryRepository, StatsRepository},
        transaction::RepoError,
    },
    workflow::{
        player::notify_player::NotifyPlayerWorkflow,
        tournament::tournament_match::TournamentMatchWorkflow,
    },
};
use tak_core::{TakFinishedGame, TakGameResult, TakPlayer};
use tak_domain::{
    GameId, MatchId,
    game::{FinishedGame, Game, GameMetadata, GameRatingInfo, OngoingGame},
    matches::MatchStatus,
    rating::{PlayerRating, RatingService},
    spectator::SpectatorService,
    stats::{GameOutcome, PlayerStats, RatingHistoryEntry},
};

#[async_trait::async_trait]
pub trait FinalizeGameWorkflow<RpCtx> {
    async fn finalize_game(
        &self,
        ctx: &RpCtx,
        game_id: GameId,
        game: OngoingGame,
        ended_game: TakFinishedGame,
    ) -> Result<(), ()>;
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
    async fn handle_match(
        &self,
        ctx: &Ports::TrCtx,
        game_id: GameId,
        match_id: MatchId,
        game: &FinishedGame,
    ) -> Result<(), String> {
        let match_repo = self.ports.match_repository();
        let tournament_match_workflow = self.tournament_match_workflow.clone();

        let mut match_data = match match_repo.get_match(ctx, match_id).await {
            Ok(Some(data)) => data,
            Ok(None) => {
                tracing::error!("Match {} not found", match_id);
                return Err("Match not found".to_string());
            }
            Err(RepoError(e)) => {
                tracing::error!("Failed to get match {}: {}", match_id, e);
                return Err("Failed to get match".to_string());
            }
        };

        match game.result {
            TakGameResult::Draw => match_data.end_game_in_match(None),
            TakGameResult::Win { winner, .. } => {
                let winner_id = match winner {
                    TakPlayer::White => game.metadata.white.user_id,
                    TakPlayer::Black => game.metadata.black.user_id,
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
                    return Err("Winner ID does not match any player in the match".to_string());
                };
                match_data.end_game_in_match(Some(is_player_one));
            }
            TakGameResult::Aborted => match_data.abort_game_in_match(),
        };
        tracing::info!("Finalizing game {} in match {}", game_id, match_id);
        match match_repo
            .update_match(ctx, match_id, match_data.clone())
            .await
        {
            Ok(_) => tracing::info!("Match {} updated successfully", match_id),
            Err(RepoError(e)) => {
                tracing::error!("Failed to update match {}: {}", match_id, e);
                return Err("Failed to update match".to_string());
            }
        }
        if let MatchStatus::Completed = match_data.status {
            tournament_match_workflow
                .handle_completed_match(ctx, match_data)
                .await
                .map_err(|RepoError(e)| {
                    tracing::error!("Failed to handle completed match {}: {}", match_id, e);
                    "Failed to handle completed match".to_string()
                })?;
        }
        Ok(())
    }

    async fn update_ratings(
        &self,
        ctx: &Ports::TrCtx,
        metadata: &GameMetadata,
        ended_game: &TakFinishedGame,
    ) -> Result<Option<GameRatingInfo>, RepoError> {
        let white_id = metadata.white.user_id;
        let black_id = metadata.black.user_id;
        let date = metadata.date;

        let rating_repo = self.ports.rating_repository();
        let (w_rating, b_rating) = futures::try_join!(
            rating_repo.get_player_rating(ctx, white_id),
            rating_repo.get_player_rating(ctx, black_id)
        )?;

        let prev_white_rating = w_rating.as_ref().map(|r| r.rating);
        let prev_black_rating = b_rating.as_ref().map(|r| r.rating);

        let mut w_rating = w_rating.unwrap_or(PlayerRating::new(white_id));
        let mut b_rating = b_rating.unwrap_or(PlayerRating::new(black_id));
        let res = self.rating_service.calculate_ratings(
            metadata,
            ended_game,
            &mut w_rating,
            &mut b_rating,
        );
        let white_rating_if_changed =
            Some(w_rating.rating).filter(|&r| Some(r) != prev_white_rating);
        let black_rating_if_changed =
            Some(b_rating.rating).filter(|&r| Some(r) != prev_black_rating);
        let info = res.map(|info| info);

        let (res_white, res_black) = futures::join!(
            rating_repo.set_player_rating(ctx, white_id, w_rating.clone()),
            rating_repo.set_player_rating(ctx, black_id, b_rating.clone())
        );
        res_white?;
        res_black?;

        futures::try_join!(
            async move {
                if let Some(new_white_rating) = &white_rating_if_changed {
                    self.ports
                        .rating_history_repository()
                        .add_rating_history_entry(
                            ctx,
                            white_id,
                            RatingHistoryEntry::new(date, *new_white_rating),
                        )
                        .await?;
                }
                Ok(())
            },
            async move {
                if let Some(new_black_rating) = &black_rating_if_changed {
                    self.ports
                        .rating_history_repository()
                        .add_rating_history_entry(
                            ctx,
                            black_id,
                            RatingHistoryEntry::new(date, *new_black_rating),
                        )
                        .await?;
                }
                Ok(())
            }
        )?;
        Ok(info)
    }

    async fn update_stats(
        &self,
        ctx: &Ports::TrCtx,
        ended_game: &FinishedGame,
        update_white: bool,
        update_black: bool,
    ) -> Result<(), RepoError> {
        let (white_outcome, black_outcome) = match ended_game.result {
            TakGameResult::Draw => (GameOutcome::Draw, GameOutcome::Draw),
            TakGameResult::Win {
                winner: TakPlayer::White,
                ..
            } => (GameOutcome::Win, GameOutcome::Loss),
            TakGameResult::Win {
                winner: TakPlayer::Black,
                ..
            } => (GameOutcome::Loss, GameOutcome::Win),
            TakGameResult::Aborted => return Ok(()),
        };

        let is_rated = ended_game.metadata.is_rated;
        let white_id = ended_game.metadata.white.user_id;
        let black_id = ended_game.metadata.black.user_id;
        futures::try_join!(
            async move {
                if update_white {
                    let stats = self
                        .ports
                        .stats_repository()
                        .get_player_stats(ctx, white_id)
                        .await?;
                    let updated = Self::update_stats_fn(stats, white_outcome, is_rated);
                    self.ports
                        .stats_repository()
                        .set_player_stats(ctx, white_id, updated)
                        .await?;
                }
                Ok(())
            },
            async move {
                if update_black {
                    let stats = self
                        .ports
                        .stats_repository()
                        .get_player_stats(ctx, black_id)
                        .await?;
                    let updated = Self::update_stats_fn(stats, black_outcome, is_rated);
                    self.ports
                        .stats_repository()
                        .set_player_stats(ctx, black_id, updated)
                        .await?;
                }
                Ok(())
            }
        )?;
        Ok(())
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
> FinalizeGameWorkflow<Ports::TrCtx> for FinalizeGameWorkflowImpl<Ports, R, NP, SPS, L, TM>
{
    #[tracing::instrument(skip(self, ctx, game, ended_game))]
    async fn finalize_game(
        &self,
        ctx: &Ports::TrCtx,
        game_id: GameId,
        game: OngoingGame,
        ended_game: TakFinishedGame,
    ) -> Result<(), ()> {
        tracing::info!("Finalizing game {}", game_id);
        let over_msg = ListenerMessage::GameEvent {
            game_id,
            event_type: ListenerGameMessageType::GameOver {
                game_result: ended_game.game_result().clone(),
            },
            time_info: ended_game.get_time_info(),
        };

        self.notify_player_workflow
            .notify_players(
                &[game.metadata.white.user_id, game.metadata.black.user_id],
                &over_msg,
            )
            .await;

        let observers = self.spectator_service.remove_game(game_id);
        self.listener_notification_port
            .notify_listeners(&observers, &over_msg);

        tracing::debug!(
            "Notified players and spectators about game {} ending, updating ratings and stats",
            game_id
        );
        let moderation_repo = self.ports.moderation_repository();
        let (white_account, black_account) = futures::try_join!(
            moderation_repo.get_moderation(&ctx, game.metadata.white.user_id),
            moderation_repo.get_moderation(&ctx, game.metadata.black.user_id)
        )
        .map_err(|RepoError(e)| {
            tracing::error!(
                "Failed to retrieve moderation info for game {}: {}",
                game_id,
                e
            );
        })?;
        let is_white_not_guest = white_account.as_ref().map_or(false, |x| !x.is_guest());
        let is_black_not_guest = black_account.as_ref().map_or(false, |x| !x.is_guest());

        let game_rating_info = if !(is_white_not_guest && is_black_not_guest) {
            None
        } else {
            self.update_ratings(&ctx, &game.metadata, &ended_game)
                .await
                .map_err(|RepoError(e)| {
                    tracing::error!("Failed to update ratings for game {}: {}", game_id, e);
                })?
        };

        let finished_game =
            FinishedGame::new(game.metadata, ended_game, game_rating_info, game.events);

        tracing::debug!(
            "Ratings updated for game {}, updating match and game history",
            game_id
        );

        if let Some(match_id) = finished_game.metadata.match_id {
            self.handle_match(&ctx, game_id, match_id, &finished_game)
                .await
                .map_err(|e| {
                    tracing::error!(
                        "Failed to update match {} for game {}: {}",
                        match_id,
                        game_id,
                        e
                    );
                })?;
        } else {
            tracing::info!("Game {} is not part of a match", game_id);
        }

        self.update_stats(&ctx, &finished_game, is_white_not_guest, is_black_not_guest)
            .await
            .map_err(|RepoError(e)| {
                tracing::error!("Failed to update stats for game {}: {}", game_id, e);
            })?;

        tracing::debug!("Stats updated for game {}", game_id);

        self.ports
            .game_repository()
            .update_game(&ctx, game_id, Game::Finished(finished_game.clone()))
            .await
            .map_err(|RepoError(e)| {
                tracing::error!(
                    "Failed to update finished game record for game {}: {}",
                    game_id,
                    e
                );
            })?;

        let ended_msg = ListenerMessage::GameEnded {
            game_id,
            game: finished_game,
        };
        self.listener_notification_port.notify_all(&ended_msg);

        tracing::debug!("Finished finalizing game {}", game_id);
        Ok(())
    }
}
