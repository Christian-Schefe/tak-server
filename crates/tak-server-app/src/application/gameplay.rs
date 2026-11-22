use std::{
    sync::Arc,
    time::{Instant, SystemTime},
};

use crate::{
    PortsCollection,
    errors::{
        AppResult, GameCantAbort, GameNotFound, GameNotOngoing, GameRequestNotFound, InternalError,
        InvalidGameAction, UnexpectedPlayer,
    },
    ports::{
        PaginatedResponse,
        game::{GameHistoryRepository, GameQuery, GameRepository, GameStateRepository},
        notification::{ListenerGameMessageType, ListenerMessage},
        spectator::SpectatorService,
        transaction::TransactionManager,
    },
    workflow::{lifecycle::LifecycleGameWorkflow, notify_player::NotifyPlayerWorkflow},
};
use tak_core::{InvalidUndoReason, TakAction};
use tak_domain::{
    GameId, ListenerId, MatchId, UserId,
    game::{
        CheckDisconnectTimeoutResult, GameAbortError, GameAcceptRequestError,
        GameCheckTimeoutError, GameDoActionError, GameEvent, GameRequest, GameRequestType,
        GameResignError, GameSetRequestError, GameState, GameUndoActionError,
    },
    game_record::GameRecord,
};
use better_errors::OneOf;

pub enum ObserveOutcome {
    Finished,
    Continue(std::time::Duration),
}

#[derive(Debug, Clone)]
pub struct GameDetailView {
    pub record: GameRecord,
    pub state: Option<GameState>,
    pub events: Vec<GameEvent>,
}

#[async_trait::async_trait]
pub trait GameplayUseCase {
    async fn get_game(
        &self,
        game_id: GameId,
    ) -> AppResult<Option<GameDetailView>, (InternalError,)>;
    async fn query_games(
        &self,
        filter: GameQuery,
    ) -> AppResult<PaginatedResponse<(GameId, GameRecord)>, (InternalError,)>;
    async fn get_games_of_match(
        &self,
        match_id: MatchId,
    ) -> AppResult<Vec<(GameId, GameRecord)>, (InternalError,)>;
    async fn get_ongoing_games(&self) -> AppResult<Vec<(GameId, GameRecord)>, (InternalError,)>;
    async fn do_action(
        &self,
        game_id: GameId,
        user_id: UserId,
        action: TakAction,
    ) -> AppResult<
        (),
        (
            InvalidGameAction,
            GameNotFound,
            GameNotOngoing,
            UnexpectedPlayer,
            InternalError,
        ),
    >;
    async fn set_request(
        &self,
        game_id: GameId,
        user_id: UserId,
        request: GameRequest,
    ) -> AppResult<
        (),
        (
            UnexpectedPlayer,
            GameNotFound,
            GameNotOngoing,
            InternalError,
        ),
    >;
    async fn accept_request(
        &self,
        game_id: GameId,
        user_id: UserId,
        request_type: GameRequestType,
    ) -> AppResult<
        (),
        (
            GameRequestNotFound,
            UnexpectedPlayer,
            GameNotFound,
            GameNotOngoing,
            InternalError,
        ),
    >;
    async fn resign(
        &self,
        game_id: GameId,
        user_id: UserId,
    ) -> AppResult<
        (),
        (
            UnexpectedPlayer,
            GameNotFound,
            GameNotOngoing,
            InternalError,
        ),
    >;
    async fn abort(
        &self,
        game_id: GameId,
        user_id: UserId,
    ) -> AppResult<
        (),
        (
            GameCantAbort,
            UnexpectedPlayer,
            GameNotFound,
            GameNotOngoing,
            InternalError,
        ),
    >;
    async fn check_player_timeout(
        &self,
        user_id: UserId,
        disconnected_at: Instant,
    ) -> AppResult<
        ObserveOutcome,
        (
            UnexpectedPlayer,
            GameNotFound,
            GameNotOngoing,
            InternalError,
        ),
    >;
    async fn observe_game(
        &self,
        game_id: GameId,
        listener_id: ListenerId,
    ) -> AppResult<(), (GameNotFound, InternalError)>;
    fn unobserve_game(&self, game_id: GameId, listener_id: ListenerId);
}

pub struct GameplayUseCaseImpl<
    Ports: PortsCollection,
    NP: NotifyPlayerWorkflow,
    F: LifecycleGameWorkflow<Ports::TrCtx>,
    S: SpectatorService,
> {
    ports: Arc<Ports>,
    notify_player_workflow: Arc<NP>,
    finalize_game_workflow: Arc<F>,
    spectator_service: Arc<S>,
}

impl<
    Ports: PortsCollection,
    NP: NotifyPlayerWorkflow,
    F: LifecycleGameWorkflow<Ports::TrCtx>,
    S: SpectatorService,
> GameplayUseCaseImpl<Ports, NP, F, S>
{
    pub fn new(
        ports: Arc<Ports>,
        notify_player_workflow: Arc<NP>,
        finalize_game_workflow: Arc<F>,
        spectator_service: Arc<S>,
    ) -> Self {
        Self {
            ports,
            notify_player_workflow,
            finalize_game_workflow,
            spectator_service,
        }
    }
}

#[async_trait::async_trait]
impl<
    Ports: PortsCollection + Send + Sync + 'static,
    NP: NotifyPlayerWorkflow + Send + Sync + 'static,
    F: LifecycleGameWorkflow<Ports::TrCtx> + Send + Sync + 'static,
    S: SpectatorService + Send + Sync + 'static,
> GameplayUseCase for GameplayUseCaseImpl<Ports, NP, F, S>
{
    async fn query_games(
        &self,
        filter: GameQuery,
    ) -> AppResult<PaginatedResponse<(GameId, GameRecord)>, (InternalError,)> {
        let ctx = self.ports.transaction_manager().simple();
        let results = self
            .ports
            .game_repository()
            .query_games(&ctx, filter)
            .await?;
        Ok(results.map(|(id, record)| (id, record)))
    }

    async fn get_game(
        &self,
        game_id: GameId,
    ) -> AppResult<Option<GameDetailView>, (InternalError,)> {
        let ctx = self.ports.transaction_manager().simple();
        let Some(game) = self.ports.game_repository().get_game(&ctx, game_id).await? else {
            return Ok(None);
        };
        let state = self
            .ports
            .game_state_repository()
            .get_state(&ctx, game_id)
            .await?;
        let events = self
            .ports
            .game_history_repository()
            .get_events(&ctx, game_id)
            .await?;
        Ok(Some(GameDetailView {
            record: game,
            state,
            events,
        }))
    }

    async fn get_games_of_match(
        &self,
        match_id: MatchId,
    ) -> AppResult<Vec<(GameId, GameRecord)>, (InternalError,)> {
        let ctx = self.ports.transaction_manager().simple();
        self.ports
            .game_repository()
            .get_games_of_match(&ctx, match_id)
            .await
    }

    async fn get_ongoing_games(&self) -> AppResult<Vec<(GameId, GameRecord)>, (InternalError,)> {
        let ctx = self.ports.transaction_manager().simple();
        self.ports.game_repository().get_ongoing_games(&ctx).await
    }

    async fn do_action(
        &self,
        game_id: GameId,
        user_id: UserId,
        action: TakAction,
    ) -> AppResult<
        (),
        (
            InvalidGameAction,
            GameNotFound,
            GameNotOngoing,
            UnexpectedPlayer,
            InternalError,
        ),
    > {
        let now = SystemTime::now();
        tracing::debug!(
            "Player {} is performing action {:?} in game {}",
            user_id,
            action,
            game_id
        );
        self.finalize_game_workflow
            .modify_game(game_id, |game, metadata| {
                let Some(player) = metadata.get_player(user_id) else {
                    return Err(OneOf::new(UnexpectedPlayer));
                };
                if let Err(e) = game.do_action(player, action, now) {
                    match e {
                        GameDoActionError::InvalidAction(_) => {
                            return Err(OneOf::new(InvalidGameAction));
                        }
                        GameDoActionError::NotPlayersTurn => {
                            return Err(OneOf::new(InvalidGameAction));
                        }
                        GameDoActionError::Timeout => {}
                    }
                };

                Ok(())
            })
            .await
            .map_err(OneOf::broaden)??;
        Ok(())
    }

    async fn set_request(
        &self,
        game_id: GameId,
        user_id: UserId,
        request: GameRequest,
    ) -> AppResult<
        (),
        (
            UnexpectedPlayer,
            GameNotFound,
            GameNotOngoing,
            InternalError,
        ),
    > {
        let now = SystemTime::now();
        let mut message = None;
        self.finalize_game_workflow
            .modify_game(game_id, |game, metadata| {
                let Some(player) = metadata.get_player(user_id) else {
                    return Err(OneOf::new(UnexpectedPlayer));
                };
                match game.set_request(player, request.clone(), now) {
                    Err(GameSetRequestError::Timeout) => Ok(()),
                    Err(GameSetRequestError::RequestAlreadySet) => Ok(()),
                    Ok(()) => {
                        let time_info = game.game.get_time_info(now);
                        message = Some((
                            ListenerMessage::GameEvent {
                                game_id,
                                event_type: ListenerGameMessageType::GameRequestChanged {
                                    user_id,
                                    request,
                                },
                                time_info,
                            },
                            metadata.white.user_id,
                            metadata.black.user_id,
                        ));
                        Ok(())
                    }
                }
            })
            .await
            .map_err(OneOf::broaden)??;
        if let Some((msg, white_user_id, black_user_id)) = message {
            self.notify_player_workflow
                .notify_players_and_observers_of_game(
                    game_id,
                    &[white_user_id, black_user_id],
                    &msg,
                );
        }
        Ok(())
    }

    async fn accept_request(
        &self,
        game_id: GameId,
        user_id: UserId,
        request_type: GameRequestType,
    ) -> AppResult<
        (),
        (
            GameRequestNotFound,
            UnexpectedPlayer,
            GameNotFound,
            GameNotOngoing,
            InternalError,
        ),
    > {
        tracing::info!(
            "Player {} is accepting request of type {:?} in game {}",
            user_id,
            request_type,
            game_id
        );
        let now = SystemTime::now();
        let mut messages = Vec::new();
        self.finalize_game_workflow
            .modify_game(game_id, |game, metadata| {
                let Some(player) = metadata.get_player(user_id) else {
                    return Err(OneOf::new(UnexpectedPlayer));
                };
                let opponent_id = metadata.get_user_id(player.opponent());
                match request_type {
                    GameRequestType::Draw => match game.accept_draw_request(player, now) {
                        Err(GameAcceptRequestError::Timeout) => Ok(()),
                        Err(GameAcceptRequestError::NoRequestToAccept) => {
                            return Err(OneOf::new(GameRequestNotFound));
                        }
                        Ok(()) => {
                            let time_info = game.game.get_time_info(now);
                            messages.push((
                                ListenerMessage::GameEvent {
                                    game_id,
                                    event_type: ListenerGameMessageType::GameRequestChanged {
                                        user_id: opponent_id,
                                        request: GameRequest::Draw(false),
                                    },
                                    time_info,
                                },
                                metadata.white.user_id,
                                metadata.black.user_id,
                            ));
                            Ok(())
                        }
                    },
                    GameRequestType::Undo => match game.accept_undo_request(player, now) {
                        Err(GameUndoActionError::Timeout) => Ok(()),
                        Err(GameUndoActionError::NoUndoAvailable) => {
                            return Err(OneOf::new(GameRequestNotFound));
                        }
                        Err(GameUndoActionError::FailedToUndo(
                            InvalidUndoReason::FailedToReconstructGame,
                        )) => {
                            tracing::error!(
                                "Failed to undo action in game {}: failed to reconstruct game",
                                game_id
                            );
                            return Err(OneOf::new(InternalError));
                        }
                        Ok(())
                        | Err(GameUndoActionError::FailedToUndo(InvalidUndoReason::NoHistory)) => {
                            let time_info = game.game.get_time_info(now);
                            messages.push((
                                ListenerMessage::GameEvent {
                                    game_id,
                                    event_type: ListenerGameMessageType::GameRequestChanged {
                                        user_id: opponent_id,
                                        request: GameRequest::Undo(false),
                                    },
                                    time_info: time_info.clone(),
                                },
                                metadata.white.user_id,
                                metadata.black.user_id,
                            ));
                            Ok(())
                        }
                    },
                    GameRequestType::MoreTime => match game.accept_more_time_request(player, now) {
                        Err(GameAcceptRequestError::Timeout) => Ok(()),
                        Err(GameAcceptRequestError::NoRequestToAccept) => {
                            return Err(OneOf::new(GameRequestNotFound));
                        }
                        Ok(()) => {
                            let time_info = game.game.get_time_info(now);
                            messages.push((
                                ListenerMessage::GameEvent {
                                    game_id,
                                    event_type: ListenerGameMessageType::GameRequestChanged {
                                        user_id: opponent_id,
                                        request: GameRequest::MoreTime(None),
                                    },
                                    time_info,
                                },
                                metadata.white.user_id,
                                metadata.black.user_id,
                            ));
                            Ok(())
                        }
                    },
                }
            })
            .await
            .map_err(OneOf::broaden)??;
        for (msg, white_user_id, black_user_id) in messages {
            self.notify_player_workflow
                .notify_players_and_observers_of_game(
                    game_id,
                    &[white_user_id, black_user_id],
                    &msg,
                );
        }
        Ok(())
    }

    async fn resign(
        &self,
        game_id: GameId,
        user_id: UserId,
    ) -> AppResult<
        (),
        (
            UnexpectedPlayer,
            GameNotFound,
            GameNotOngoing,
            InternalError,
        ),
    > {
        let now = SystemTime::now();
        self.finalize_game_workflow
            .modify_game(game_id, |game, metadata| {
                let Some(player) = metadata.get_player(user_id) else {
                    return Err(OneOf::new(UnexpectedPlayer));
                };
                match game.resign(player, now) {
                    Err(GameResignError::Timeout) => Ok(()),
                    Ok(()) => Ok(()),
                }
            })
            .await
            .map_err(OneOf::broaden)??;
        Ok(())
    }

    async fn abort(
        &self,
        game_id: GameId,
        user_id: UserId,
    ) -> AppResult<
        (),
        (
            GameCantAbort,
            UnexpectedPlayer,
            GameNotFound,
            GameNotOngoing,
            InternalError,
        ),
    > {
        let now = SystemTime::now();
        self.finalize_game_workflow
            .modify_game(game_id, |game, metadata| {
                let Some(_) = metadata.get_player(user_id) else {
                    return Err(OneOf::new(UnexpectedPlayer));
                };
                match game.abort(now) {
                    Err(GameAbortError::Timeout) => Ok(()),
                    Err(GameAbortError::CantAbort) => {
                        return Err(OneOf::new(GameCantAbort));
                    }
                    Ok(()) => Ok(()),
                }
            })
            .await
            .map_err(OneOf::broaden)??;
        Ok(())
    }

    async fn check_player_timeout(
        &self,
        user_id: UserId,
        disconnected_at: Instant,
    ) -> AppResult<
        ObserveOutcome,
        (
            UnexpectedPlayer,
            GameNotFound,
            GameNotOngoing,
            InternalError,
        ),
    > {
        let now = SystemTime::now();
        let disconnected_duration = Instant::now().saturating_duration_since(disconnected_at);
        let ctx = self.ports.transaction_manager().simple();
        let games = self
            .ports
            .game_repository()
            .get_ongoing_games_by_player(&ctx, user_id)
            .await
            .map_err(OneOf::broaden)?;

        let mut wait_duration = None;

        for (game_id, record) in games {
            self.finalize_game_workflow
                .modify_game(game_id, |game, _| {
                    let Some(player) = record.metadata.get_player(user_id) else {
                        tracing::warn!(
                            "Player {} not found in game {} while checking player timeout",
                            user_id,
                            game_id
                        );
                        return Err(OneOf::new(UnexpectedPlayer));
                    };
                    match game.check_disconnect_timeout(player, disconnected_duration, now) {
                        Err(GameCheckTimeoutError::Timeout) => Ok(()),
                        Ok(res) => match res {
                            CheckDisconnectTimeoutResult::TimedOut => Ok(()),
                            CheckDisconnectTimeoutResult::CantTimeOut => Ok(()),
                            CheckDisconnectTimeoutResult::NoTimeout(duration) => {
                                wait_duration = match wait_duration {
                                    Some(current) if current < duration => Some(current),
                                    _ => Some(duration),
                                };
                                Ok(())
                            }
                        },
                    }
                })
                .await
                .map_err(OneOf::broaden)??;
        }

        match wait_duration {
            Some(duration) => Ok(ObserveOutcome::Continue(
                duration + std::time::Duration::from_millis(100),
            )),
            None => Ok(ObserveOutcome::Finished),
        }
    }

    async fn observe_game(
        &self,
        game_id: GameId,
        listener_id: ListenerId,
    ) -> AppResult<(), (GameNotFound, InternalError)> {
        let ctx = self.ports.transaction_manager().simple();
        let Some(_) = self
            .ports
            .game_state_repository()
            .get_state(&ctx, game_id)
            .await
            .map_err(OneOf::broaden)?
        else {
            return Err(OneOf::new(GameNotFound));
        };
        self.spectator_service.observe_game(game_id, listener_id);
        Ok(())
    }

    fn unobserve_game(&self, game_id: GameId, listener_id: ListenerId) {
        self.spectator_service.unobserve_game(game_id, listener_id);
    }
}
