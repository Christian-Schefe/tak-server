use std::sync::Arc;

use crate::{
    PortsCollection,
    ports::{
        game::{GameHistoryRepository, GameRepository, GameStateRepository},
        matches::MatchRepository,
        moderation::ModerationRepository,
        notification::{ListenerGameMessageType, ListenerMessage, ListenerNotificationPort},
        rating::RatingRepository,
        stats::{RatingHistoryRepository, StatsRepository},
        tournament::TournamentPlayerRepository,
        transaction::{RepoError, TransactionManager},
    },
    workflow::listener::notify_player::NotifyPlayerWorkflow,
};
use tak_core::{TakActionRecord, TakGameResult, TakPlayer};
use tak_domain::{
    GameId, MatchId,
    game::{Game, GameCreationError, GameEventType},
    game_record::{GameMetadata, GameRatingInfo, GameRecord},
    matches::MatchStatus,
    rating::{PlayerRating, RatingService},
    spectator::SpectatorService,
    stats::{GameOutcome, PlayerStats, RatingHistoryEntry},
};

#[async_trait::async_trait]
pub trait LifecycleGameWorkflow<RpCtx> {
    async fn modify_game<F, Err>(&self, game_id: GameId, func: F) -> Result<Result<(), Err>, ()>
    where
        F: FnOnce(&mut Game, &GameMetadata) -> Result<(), Err> + Send,
        Err: Send + Sync + 'static;
}

pub struct LifecycleGameWorkflowImpl<
    Ports: PortsCollection,
    R: RatingService,
    NP: NotifyPlayerWorkflow,
    SPS: SpectatorService,
    L: ListenerNotificationPort,
> {
    ports: Arc<Ports>,
    rating_service: Arc<R>,
    notify_player_workflow: Arc<NP>,
    spectator_service: Arc<SPS>,
    listener_notification_port: Arc<L>,
}

impl<
    Ports: PortsCollection,
    R: RatingService,
    NP: NotifyPlayerWorkflow,
    SPS: SpectatorService,
    L: ListenerNotificationPort,
> LifecycleGameWorkflowImpl<Ports, R, NP, SPS, L>
{
    pub fn new(
        ports: Arc<Ports>,
        rating_service: Arc<R>,
        notify_player_workflow: Arc<NP>,
        spectator_service: Arc<SPS>,
        listener_notification_port: Arc<L>,
    ) -> Self {
        Self {
            ports,
            rating_service,
            notify_player_workflow,
            spectator_service,
            listener_notification_port,
        }
    }
}
impl<
    Ports: PortsCollection + Send + Sync + 'static,
    R: RatingService + Send + Sync + 'static,
    NP: NotifyPlayerWorkflow + Send + Sync + 'static,
    SPS: SpectatorService + Send + Sync + 'static,
    L: ListenerNotificationPort + Send + Sync + 'static,
> LifecycleGameWorkflowImpl<Ports, R, NP, SPS, L>
{
    async fn handle_match(
        &self,
        ctx: &Ports::TrCtx,
        game_id: GameId,
        match_id: MatchId,
        metadata: &GameMetadata,
        result: TakGameResult,
    ) -> Result<(), String> {
        let match_repo = self.ports.match_repository();

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

        match result {
            TakGameResult::Draw => match_data.end_game_in_match(None),
            TakGameResult::Win { winner, .. } => {
                let winner_id = match winner {
                    TakPlayer::White => metadata.white.user_id,
                    TakPlayer::Black => metadata.black.user_id,
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
            let Some(tournament_info) = &match_data.tournament_info else {
                return Ok(());
            };
            let winner = match_data.get_winner();
            let repo = self.ports.tournament_player_repository();
            if let Some(winner_id) = winner {
                repo.increase_player_score(&ctx, tournament_info.tournament_id, winner_id, 2)
                    .await
                    .map_err(|RepoError(e)| {
                        tracing::error!("Failed to handle completed match {}: {}", match_id, e);
                        "Failed to handle completed match".to_string()
                    })?;
            } else {
                futures::try_join!(
                    repo.increase_player_score(
                        &ctx,
                        tournament_info.tournament_id,
                        match_data.player1.user_id,
                        1,
                    ),
                    repo.increase_player_score(
                        &ctx,
                        tournament_info.tournament_id,
                        match_data.player2.user_id,
                        1,
                    ),
                )
                .map_err(|RepoError(e)| {
                    tracing::error!("Failed to handle completed match {}: {}", match_id, e);
                    "Failed to handle completed match".to_string()
                })?;
            }
        }
        Ok(())
    }

    async fn update_ratings(
        &self,
        ctx: &Ports::TrCtx,
        metadata: &GameMetadata,
        result: TakGameResult,
        history: &[TakActionRecord],
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
            result,
            history,
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
        metadata: &GameMetadata,
        result: TakGameResult,
        update_white: bool,
        update_black: bool,
    ) -> Result<(), RepoError> {
        let (white_outcome, black_outcome) = match result {
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

        let is_rated = metadata.is_rated;
        let white_id = metadata.white.user_id;
        let black_id = metadata.black.user_id;
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

    #[tracing::instrument(skip(self, ctx, record))]
    async fn finalize_game(
        &self,
        ctx: &Ports::TrCtx,
        game_id: GameId,
        mut record: GameRecord,
        history: &Vec<TakActionRecord>,
        result: TakGameResult,
    ) -> Result<(), ()> {
        tracing::info!("Finalizing game {}", game_id);

        let moderation_repo = self.ports.moderation_repository();
        let (white_account, black_account) = futures::try_join!(
            moderation_repo.get_moderation(&ctx, record.metadata.white.user_id),
            moderation_repo.get_moderation(&ctx, record.metadata.black.user_id)
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

        let rating_info = if !(is_white_not_guest && is_black_not_guest) {
            None
        } else {
            self.update_ratings(&ctx, &record.metadata, result, history)
                .await
                .map_err(|RepoError(e)| {
                    tracing::error!("Failed to update ratings for game {}: {}", game_id, e);
                })?
        };

        tracing::debug!(
            "Ratings updated for game {}, updating match and game history",
            game_id
        );

        if let Some(match_id) = record.metadata.match_id {
            self.handle_match(&ctx, game_id, match_id, &record.metadata, result)
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

        self.update_stats(
            &ctx,
            &record.metadata,
            result,
            is_white_not_guest,
            is_black_not_guest,
        )
        .await
        .map_err(|RepoError(e)| {
            tracing::error!("Failed to update stats for game {}: {}", game_id, e);
        })?;

        tracing::debug!("Stats updated for game {}", game_id);

        record.update_finished(result, rating_info.clone());
        self.ports
            .game_repository()
            .update_game(&ctx, game_id, record.clone())
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
            game: record,
        };
        self.listener_notification_port.notify_all(&ended_msg);

        tracing::debug!("Finished finalizing game {}", game_id);
        Ok(())
    }
}

#[async_trait::async_trait]
impl<
    Ports: PortsCollection + Send + Sync + 'static,
    R: RatingService + Send + Sync + 'static,
    NP: NotifyPlayerWorkflow + Send + Sync + 'static,
    SPS: SpectatorService + Send + Sync + 'static,
    L: ListenerNotificationPort + Send + Sync + 'static,
> LifecycleGameWorkflow<Ports::TrCtx> for LifecycleGameWorkflowImpl<Ports, R, NP, SPS, L>
{
    async fn modify_game<F, Err>(&self, game_id: GameId, func: F) -> Result<Result<(), Err>, ()>
    where
        F: FnOnce(&mut Game, &GameMetadata) -> Result<(), Err> + Send,
        Err: Send + Sync + 'static,
    {
        let ctx = match self.ports.transaction_manager().transaction().await {
            Ok(ctx) => ctx,
            Err(RepoError(e)) => {
                tracing::error!("Failed to create transaction: {}", e);
                return Err(());
            }
        };
        let record = match self.ports.game_repository().get_game(&ctx, game_id).await {
            Ok(Some(record)) => record,
            Ok(None) => {
                tracing::error!("Game {} not found in game repository", game_id);
                return Err(());
            }
            Err(RepoError(e)) => {
                tracing::error!(
                    "Failed to retrieve game record {} from repository: {}",
                    game_id,
                    e
                );
                return Err(());
            }
        };
        let game_state = match self
            .ports
            .game_state_repository()
            .get_state(&ctx, game_id)
            .await
        {
            Ok(Some(state)) => state,
            Ok(None) => {
                tracing::error!("Game state for {} not found in repository", game_id);
                return Err(());
            }
            Err(RepoError(e)) => {
                tracing::error!(
                    "Failed to retrieve game state {} from repository: {}",
                    game_id,
                    e
                );
                return Err(());
            }
        };
        let mut game = match Game::from_state(&record.metadata.settings, game_state) {
            Ok(game) => game,
            Err(GameCreationError::InvalidState) => {
                tracing::error!("Failed to create game from state");
                return Err(());
            }
        };
        if let Err(e) = func(&mut game, &record.metadata) {
            return Ok(Err(e));
        }
        let events = game.drain_event_log();
        let new_state = game.to_game_state();

        let notify_players_and_spectators = |event: &ListenerMessage| {
            self.notify_player_workflow.notify_players(
                &[record.metadata.white.user_id, record.metadata.black.user_id],
                event,
            );
            let spectators = self.spectator_service.get_spectators_for_game(game_id);
            self.listener_notification_port
                .notify_listeners(&spectators, event);
        };

        let mut game_ended = false;
        for event in &events {
            match &event.event_type {
                GameEventType::ActionPerformed {
                    player,
                    ply_index,
                    action,
                } => {
                    let game_action = ListenerGameMessageType::GameAction {
                        user_id: record.metadata.get_user_id(*player),
                        action: action.clone(),
                        ply_index: *ply_index,
                    };
                    notify_players_and_spectators(&ListenerMessage::GameEvent {
                        game_id,
                        event_type: game_action,
                        time_info: event.time_info,
                    });
                }
                GameEventType::ActionUndone { ply_index } => {
                    let game_action = ListenerGameMessageType::GameActionUndone {
                        ply_index: *ply_index,
                    };
                    notify_players_and_spectators(&ListenerMessage::GameEvent {
                        game_id,
                        event_type: game_action,
                        time_info: event.time_info,
                    });
                }
                GameEventType::GameOver { result } => {
                    let over_msg = ListenerMessage::GameEvent {
                        game_id,
                        event_type: ListenerGameMessageType::GameOver {
                            game_result: result.clone(),
                        },
                        time_info: event.time_info,
                    };

                    self.notify_player_workflow.notify_players(
                        &[record.metadata.white.user_id, record.metadata.black.user_id],
                        &over_msg,
                    );

                    let observers = self.spectator_service.remove_game(game_id);
                    self.listener_notification_port
                        .notify_listeners(&observers, &over_msg);

                    tracing::debug!(
                        "Notified players and spectators about game {} ending, updating ratings and stats",
                        game_id
                    );

                    if let Err(()) = self
                        .finalize_game(&ctx, game_id, record, &new_state.history, *result)
                        .await
                    {
                        tracing::error!("Failed to finalize game {}", game_id);
                        return Err(());
                    }
                    game_ended = true;
                    break;
                }
            }
        }

        if let Err(RepoError(e)) = self
            .ports
            .game_history_repository()
            .add_events(&ctx, game_id, events)
            .await
        {
            tracing::error!("Failed to add game events for game {}: {}", game_id, e);
            return Err(());
        }

        if game_ended {
            if let Err(RepoError(e)) = self
                .ports
                .game_state_repository()
                .remove_state(&ctx, game_id)
                .await
            {
                tracing::error!("Failed to remove ongoing game {}: {}", game_id, e);
                return Err(());
            }
        } else {
            if let Err(RepoError(e)) = self
                .ports
                .game_state_repository()
                .set_state(&ctx, game_id, new_state)
                .await
            {
                tracing::error!("Failed to update ongoing game {}: {}", game_id, e);
                return Err(());
            }
        }

        match self.ports.transaction_manager().commit(ctx).await {
            Ok(_) => Ok(Ok(())),
            Err(RepoError(e)) => {
                tracing::error!("Failed to commit transaction: {}", e);
                Err(())
            }
        }
    }
}
