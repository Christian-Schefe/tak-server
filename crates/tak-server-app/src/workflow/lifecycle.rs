use crate::{
    PortsCollection,
    errors::{AppResult, GameNotFound, GameNotOngoing, InternalError, MatchNotFound},
    ports::notification::{NotificationGameMessageType, NotificationMessage},
};
use better_errors::OneOf;
use tak_core::{TakActionRecord, TakGameResult, TakPlayer};
use tak_domain::{
    GameId, MatchId,
    game::{Game, GameCreationError, GameEventType},
    game_record::{GameMetadata, GameRatingInfo, GameRecord},
    matches::MatchStatus,
    rating::{PlayerRating, calculate_ratings},
    stats::{GameOutcome, PlayerStats, RatingHistoryEntry},
};

async fn handle_match<Ports: PortsCollection + Send + Sync + 'static>(
    ports: &Ports,
    ctx: &Ports::RpCtx,
    game_id: GameId,
    match_id: MatchId,
    metadata: &GameMetadata,
    result: TakGameResult,
) -> AppResult<(), (InternalError, MatchNotFound)> {
    let match_repo = ports.match_repository();

    let mut match_data = match match_repo
        .get_match(ctx, match_id)
        .await
        .map_err(OneOf::broaden)?
    {
        Some(data) => data,
        None => {
            tracing::error!("Match {} not found", match_id);
            return Err(OneOf::new(MatchNotFound));
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
                return Err(OneOf::new(InternalError));
            };
            match_data.end_game_in_match(Some(is_player_one));
        }
        TakGameResult::Aborted => match_data.abort_game_in_match(),
    };
    tracing::info!("Finalizing game {} in match {}", game_id, match_id);
    match_repo
        .update_match(ctx, match_id, match_data.clone())
        .await
        .map_err(OneOf::broaden)?;
    if let MatchStatus::Completed = match_data.status {
        let Some(tournament_info) = &match_data.tournament_info else {
            return Ok(());
        };
        let winner = match_data.get_winner();
        let repo = ports.tournament_player_repository();
        if let Some(winner_id) = winner {
            repo.increase_player_score(&ctx, tournament_info.tournament_id, winner_id, 2)
                .await
                .map_err(OneOf::broaden)?;
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
            .map_err(OneOf::broaden)?;
        }
    }
    Ok(())
}

async fn update_ratings<Ports: PortsCollection + Send + Sync + 'static>(
    ports: &Ports,
    ctx: &Ports::RpCtx,
    metadata: &GameMetadata,
    result: TakGameResult,
    history: &[TakActionRecord],
) -> AppResult<Option<GameRatingInfo>, (InternalError,)> {
    let white_id = metadata.white.user_id;
    let black_id = metadata.black.user_id;
    let date = metadata.date;

    let rating_repo = ports.rating_repository();
    let (w_rating, b_rating) = futures::try_join!(
        rating_repo.get_player_rating(ctx, white_id),
        rating_repo.get_player_rating(ctx, black_id)
    )?;

    let prev_white_rating = w_rating.as_ref().map(|r| r.rating);
    let prev_black_rating = b_rating.as_ref().map(|r| r.rating);

    let mut w_rating = w_rating.unwrap_or(PlayerRating::new(white_id));
    let mut b_rating = b_rating.unwrap_or(PlayerRating::new(black_id));
    let res = calculate_ratings(metadata, result, history, &mut w_rating, &mut b_rating);
    let white_rating_if_changed = Some(w_rating.rating).filter(|&r| Some(r) != prev_white_rating);
    let black_rating_if_changed = Some(b_rating.rating).filter(|&r| Some(r) != prev_black_rating);
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
                ports
                    .rating_history_repository()
                    .add_rating_history_entry(
                        ctx,
                        white_id,
                        RatingHistoryEntry::new(date, *new_white_rating),
                    )
                    .await?;
            }
            Result::<(), OneOf<(InternalError,)>>::Ok(())
        },
        async move {
            if let Some(new_black_rating) = &black_rating_if_changed {
                ports
                    .rating_history_repository()
                    .add_rating_history_entry(
                        ctx,
                        black_id,
                        RatingHistoryEntry::new(date, *new_black_rating),
                    )
                    .await?;
            }
            Result::<(), OneOf<(InternalError,)>>::Ok(())
        }
    )?;
    Ok(info)
}

async fn update_stats<Ports: PortsCollection + Send + Sync + 'static>(
    ports: &Ports,
    ctx: &Ports::RpCtx,
    metadata: &GameMetadata,
    result: TakGameResult,
    update_white: bool,
    update_black: bool,
) -> AppResult<(), (InternalError,)> {
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
                let stats = ports
                    .stats_repository()
                    .get_player_stats(ctx, white_id)
                    .await?;
                let updated = update_stats_fn(stats, white_outcome, is_rated);
                ports
                    .stats_repository()
                    .set_player_stats(ctx, white_id, updated)
                    .await?;
            }
            Result::<(), OneOf<(InternalError,)>>::Ok(())
        },
        async move {
            if update_black {
                let stats = ports
                    .stats_repository()
                    .get_player_stats(ctx, black_id)
                    .await?;
                let updated = update_stats_fn(stats, black_outcome, is_rated);
                ports
                    .stats_repository()
                    .set_player_stats(ctx, black_id, updated)
                    .await?;
            }
            Result::<(), OneOf<(InternalError,)>>::Ok(())
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

#[tracing::instrument(skip(ports, ctx, record))]
async fn finalize_game<Ports: PortsCollection + Send + Sync + 'static>(
    ports: &Ports,
    ctx: &Ports::RpCtx,
    game_id: GameId,
    mut record: GameRecord,
    history: &Vec<TakActionRecord>,
    result: TakGameResult,
) -> AppResult<(), (InternalError,)> {
    tracing::info!("Finalizing game {}", game_id);

    let moderation_repo = ports.moderation_repository();
    let (white_account, black_account) = futures::try_join!(
        moderation_repo.get_moderation(&ctx, record.metadata.white.user_id),
        moderation_repo.get_moderation(&ctx, record.metadata.black.user_id)
    )
    .map_err(OneOf::broaden)?;
    let is_white_not_guest = white_account.as_ref().map_or(false, |x| !x.is_guest());
    let is_black_not_guest = black_account.as_ref().map_or(false, |x| !x.is_guest());

    let rating_info = if !(is_white_not_guest && is_black_not_guest) {
        None
    } else {
        update_ratings(ports, &ctx, &record.metadata, result, history)
            .await
            .map_err(OneOf::broaden)?
    };

    tracing::debug!(
        "Ratings updated for game {}, updating match and game history",
        game_id
    );

    if let Some(match_id) = record.metadata.match_id {
        handle_match(ports, &ctx, game_id, match_id, &record.metadata, result)
            .await
            .map_err(|e| match OneOf::narrow(e) {
                Ok(MatchNotFound) => {
                    tracing::error!(
                        "Match {} not found while finalizing game {}",
                        match_id,
                        game_id
                    );
                    return OneOf::new(InternalError);
                }
                Err(e) => e,
            })?;
    } else {
        tracing::info!("Game {} is not part of a match", game_id);
    }

    update_stats(
        ports,
        &ctx,
        &record.metadata,
        result,
        is_white_not_guest,
        is_black_not_guest,
    )
    .await
    .map_err(OneOf::broaden)?;

    tracing::debug!("Stats updated for game {}", game_id);

    record.update_finished(result, rating_info.clone());
    ports
        .game_repository()
        .update_game(&ctx, game_id, record.clone())
        .await
        .map_err(OneOf::broaden)?;

    let ended_msg = NotificationMessage::GameEnded {
        game_id,
        game: record,
    };
    ports.notification_port().notify(&ended_msg);

    tracing::debug!("Finished finalizing game {}", game_id);
    Ok(())
}

pub async fn modify_game<Ports: PortsCollection + Send + Sync + 'static, F, Res, Err>(
    ports: &Ports,
    ctx: &Ports::RpCtx,
    game_id: GameId,
    func: F,
) -> AppResult<Result<Res, Err>, (GameNotFound, GameNotOngoing, InternalError)>
where
    F: FnOnce(&mut Game, &GameMetadata) -> Result<Res, Err> + Send,
    Err: Send + Sync + 'static,
    Res: Send + Sync + 'static,
{
    let Some(record) = ports
        .game_repository()
        .get_game(&ctx, game_id)
        .await
        .map_err(OneOf::broaden)?
    else {
        return Err(OneOf::new(GameNotFound));
    };
    let game_state = match ports
        .game_state_repository()
        .get_state(&ctx, game_id)
        .await
        .map_err(OneOf::broaden)?
    {
        Some(state) => state,
        None => {
            tracing::error!("Game state for {} not found in repository", game_id);
            return Err(OneOf::new(GameNotOngoing));
        }
    };
    let mut game = match Game::from_state(&record.metadata.settings, game_state) {
        Ok(game) => game,
        Err(GameCreationError::InvalidState) => {
            tracing::error!("Failed to create game from state");
            return Err(OneOf::new(InternalError));
        }
    };
    let res = match func(&mut game, &record.metadata) {
        Ok(res) => res,
        Err(e) => {
            return Ok(Err(e));
        }
    };
    let events = game.drain_event_log();
    let new_state = game.to_game_state();

    let mut game_ended = false;
    for event in &events {
        match &event.event_type {
            GameEventType::RequestSet { player, request } => {
                let game_request = NotificationGameMessageType::GameRequestChanged {
                    player: *player,
                    request: request.clone(),
                };
                ports
                    .notification_port()
                    .notify(&NotificationMessage::GameEvent {
                        game_id,
                        event_type: game_request,
                        time_info: event.time_info,
                    });
            }
            GameEventType::ActionPerformed { ply_index, action } => {
                let game_action = NotificationGameMessageType::GameAction {
                    action: action.clone(),
                    ply_index: *ply_index,
                };
                ports
                    .notification_port()
                    .notify(&NotificationMessage::GameEvent {
                        game_id,
                        event_type: game_action,
                        time_info: event.time_info,
                    });
            }
            GameEventType::ActionUndone { ply_index } => {
                let game_action = NotificationGameMessageType::GameActionUndone {
                    ply_index: *ply_index,
                };
                ports
                    .notification_port()
                    .notify(&NotificationMessage::GameEvent {
                        game_id,
                        event_type: game_action,
                        time_info: event.time_info,
                    });
            }
            GameEventType::GameOver { result } => {
                ports
                    .notification_port()
                    .notify(&NotificationMessage::GameEvent {
                        game_id,
                        event_type: NotificationGameMessageType::GameOver {
                            game_result: result.clone(),
                        },
                        time_info: event.time_info,
                    });

                tracing::debug!(
                    "Notified players and spectators about game {} ending, updating ratings and stats",
                    game_id
                );

                finalize_game(ports, &ctx, game_id, record, &new_state.history, *result)
                    .await
                    .map_err(OneOf::broaden)?;
                game_ended = true;
                break;
            }
        }
    }

    ports
        .game_history_repository()
        .add_events(&ctx, game_id, events)
        .await
        .map_err(OneOf::broaden)?;

    if game_ended {
        ports
            .game_state_repository()
            .remove_state(&ctx, game_id)
            .await
            .map_err(OneOf::broaden)?;
    } else {
        ports
            .game_state_repository()
            .set_state(&ctx, game_id, new_state)
            .await
            .map_err(OneOf::broaden)?;
    }

    Ok(Ok(res))
}
