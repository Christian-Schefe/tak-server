use std::{
    sync::Arc,
    time::{Instant, SystemTime},
};

use crate::{
    PortsCollection,
    ports::{
        game::GameRepository,
        notification::{ListenerGameMessageType, ListenerMessage},
        transaction::{RepoError, TransactionManager},
    },
    workflow::{
        gameplay::lifecycle::LifecycleGameWorkflow, listener::notify_player::NotifyPlayerWorkflow,
    },
};
use tak_core::{InvalidUndoReason, TakAction};
use tak_domain::{
    GameId, ListenerId, UserId,
    game::{
        CheckDisconnectTimeoutResult, GameAbortError, GameAcceptRequestError,
        GameCheckTimeoutError, GameDoActionError, GameRequest, GameRequestType, GameResignError,
        GameSetRequestError, GameUndoActionError,
    },
    spectator::SpectatorService,
};

pub mod lifecycle;
pub mod query;

pub enum ObserveOutcome {
    Finished,
    Continue(std::time::Duration),
}

pub enum ObserveError {
    GameNotFound,
    Internal,
}

#[async_trait::async_trait]
pub trait GameplayUseCase {
    async fn do_action(
        &self,
        game_id: GameId,
        user_id: UserId,
        action: TakAction,
    ) -> Result<(), ActionError<DoActionError>>;
    async fn set_request(
        &self,
        game_id: GameId,
        user_id: UserId,
        request: GameRequest,
    ) -> Result<(), PlayerActionError>;
    async fn accept_request(
        &self,
        game_id: GameId,
        user_id: UserId,
        request_type: GameRequestType,
    ) -> Result<(), ActionError<HandleRequestError>>;
    async fn resign(&self, game_id: GameId, user_id: UserId) -> Result<(), PlayerActionError>;
    async fn abort(&self, game_id: GameId, user_id: UserId) -> Result<(), ActionError<AbortError>>;

    async fn check_player_timeout(
        &self,
        user_id: UserId,
        disconnected_at: Instant,
    ) -> Result<ObserveOutcome, ObserveError>;
    async fn observe_game(
        &self,
        game_id: GameId,
        listener_id: ListenerId,
    ) -> Result<(), ObserveGameError>;
    fn unobserve_game(&self, game_id: GameId, listener_id: ListenerId);
}

#[derive(Debug)]
pub enum PlayerActionError {
    GameNotFound,
    NotAPlayerInGame,
    Internal,
}

#[derive(Debug)]
pub enum ActionError<R> {
    NotPossible(PlayerActionError),
    ActionError(R),
}

#[derive(Debug)]
pub enum DoActionError {
    InvalidAction(tak_core::InvalidActionReason),
    NotPlayersTurn,
}

#[derive(Debug)]
pub enum HandleRequestError {
    RequestNotFound,
}

pub enum AbortError {
    GameAlreadyStarted,
}

pub enum ObserveGameError {
    GameNotFound,
    Internal,
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
    async fn do_action(
        &self,
        game_id: GameId,
        user_id: UserId,
        action: TakAction,
    ) -> Result<(), ActionError<DoActionError>> {
        let now = SystemTime::now();
        tracing::debug!(
            "Player {} is performing action {:?} in game {}",
            user_id,
            action,
            game_id
        );
        match self
            .finalize_game_workflow
            .modify_game(game_id, |game, metadata| {
                let Some(player) = metadata.get_player(user_id) else {
                    return Err(ActionError::NotPossible(
                        PlayerActionError::NotAPlayerInGame,
                    ));
                };
                if let Err(e) = game.do_action(player, action, now) {
                    match e {
                        GameDoActionError::InvalidAction(e) => {
                            return Err(ActionError::ActionError(DoActionError::InvalidAction(e)));
                        }
                        GameDoActionError::NotPlayersTurn => {
                            return Err(ActionError::ActionError(DoActionError::NotPlayersTurn));
                        }
                        GameDoActionError::Timeout => {}
                    }
                };

                Ok(())
            })
            .await
        {
            Ok(Err(e)) => Err(e),
            Ok(Ok(())) => Ok(()),
            Err(()) => Err(ActionError::NotPossible(PlayerActionError::Internal)),
        }
    }

    async fn set_request(
        &self,
        game_id: GameId,
        user_id: UserId,
        request: GameRequest,
    ) -> Result<(), PlayerActionError> {
        let now = SystemTime::now();
        let mut message = None;
        match self
            .finalize_game_workflow
            .modify_game(game_id, |game, metadata| {
                let Some(player) = metadata.get_player(user_id) else {
                    return Err(PlayerActionError::NotAPlayerInGame);
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
        {
            Ok(Err(e)) => return Err(e),
            Ok(Ok(())) => {}
            Err(()) => return Err(PlayerActionError::Internal),
        }
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
    ) -> Result<(), ActionError<HandleRequestError>> {
        tracing::info!(
            "Player {} is accepting request of type {:?} in game {}",
            user_id,
            request_type,
            game_id
        );
        let now = SystemTime::now();
        let mut messages = Vec::new();
        match self
            .finalize_game_workflow
            .modify_game(game_id, |game, metadata| {
                let Some(player) = metadata.get_player(user_id) else {
                    return Err(ActionError::NotPossible(
                        PlayerActionError::NotAPlayerInGame,
                    ));
                };
                let opponent_id = metadata.get_user_id(player.opponent());
                match request_type {
                    GameRequestType::Draw => match game.accept_draw_request(player, now) {
                        Err(GameAcceptRequestError::Timeout) => Ok(()),
                        Err(GameAcceptRequestError::NoRequestToAccept) => {
                            return Err(ActionError::ActionError(
                                HandleRequestError::RequestNotFound,
                            ));
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
                            return Err(ActionError::ActionError(
                                HandleRequestError::RequestNotFound,
                            ));
                        }
                        Err(GameUndoActionError::FailedToUndo(
                            InvalidUndoReason::FailedToReconstructGame,
                        )) => {
                            tracing::error!(
                                "Failed to undo action in game {}: failed to reconstruct game",
                                game_id
                            );
                            return Err(ActionError::NotPossible(PlayerActionError::Internal));
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
                            return Err(ActionError::ActionError(
                                HandleRequestError::RequestNotFound,
                            ));
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
        {
            Ok(Err(e)) => return Err(e),
            Ok(Ok(())) => {}
            Err(()) => return Err(ActionError::NotPossible(PlayerActionError::Internal)),
        }
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

    async fn resign(&self, game_id: GameId, user_id: UserId) -> Result<(), PlayerActionError> {
        let now = SystemTime::now();
        match self
            .finalize_game_workflow
            .modify_game(game_id, |game, metadata| {
                let Some(player) = metadata.get_player(user_id) else {
                    return Err(PlayerActionError::NotAPlayerInGame);
                };
                match game.resign(player, now) {
                    Err(GameResignError::Timeout) => Ok(()),
                    Ok(()) => Ok(()),
                }
            })
            .await
        {
            Ok(res) => res,
            Err(()) => return Err(PlayerActionError::Internal),
        }
    }

    async fn abort(&self, game_id: GameId, user_id: UserId) -> Result<(), ActionError<AbortError>> {
        let now = SystemTime::now();
        match self
            .finalize_game_workflow
            .modify_game(game_id, |game, metadata| {
                let Some(_) = metadata.get_player(user_id) else {
                    return Err(ActionError::NotPossible(
                        PlayerActionError::NotAPlayerInGame,
                    ));
                };
                match game.abort(now) {
                    Err(GameAbortError::Timeout) => Ok(()),
                    Err(GameAbortError::CantAbort) => {
                        return Err(ActionError::ActionError(AbortError::GameAlreadyStarted));
                    }
                    Ok(()) => Ok(()),
                }
            })
            .await
        {
            Ok(Err(e)) => return Err(e),
            Ok(Ok(())) => Ok(()),
            Err(()) => return Err(ActionError::NotPossible(PlayerActionError::Internal)),
        }
    }

    async fn check_player_timeout(
        &self,
        user_id: UserId,
        disconnected_at: Instant,
    ) -> Result<ObserveOutcome, ObserveError> {
        let now = SystemTime::now();
        let disconnected_duration = Instant::now().saturating_duration_since(disconnected_at);
        let ctx = self.ports.transaction_manager().simple();
        let games = match self
            .ports
            .game_repository()
            .get_ongoing_games_by_player(&ctx, user_id)
            .await
        {
            Ok(games) => games,
            Err(RepoError(e)) => {
                tracing::error!("Error retrieving games for player {}: {}", user_id, e);
                return Err(ObserveError::Internal);
            }
        };

        let mut wait_duration = None;

        for (game_id, record) in games {
            match self
                .finalize_game_workflow
                .modify_game(game_id, |game, _| {
                    let Some(player) = record.metadata.get_player(user_id) else {
                        tracing::warn!(
                            "Player {} not found in game {} while checking player timeout",
                            user_id,
                            game_id
                        );
                        return Err(());
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
            {
                Ok(Ok(())) => {}
                Ok(Err(())) => {}
                Err(()) => {
                    tracing::error!(
                        "Error checking disconnect timeout for player {} in game {}",
                        user_id,
                        game_id
                    );
                    return Err(ObserveError::Internal);
                }
            }
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
    ) -> Result<(), ObserveGameError> {
        let ctx = self.ports.transaction_manager().simple();
        match self.ports.game_repository().get_game(&ctx, game_id).await {
            Ok(None) => return Err(ObserveGameError::GameNotFound),
            Ok(Some(record)) => {
                if record.result.is_some() {
                    return Err(ObserveGameError::GameNotFound);
                }
            }
            Err(RepoError(e)) => {
                tracing::error!("Failed to retrieve game {} from repository: {}", game_id, e);
                return Err(ObserveGameError::Internal);
            }
        }
        self.spectator_service.observe_game(game_id, listener_id);
        Ok(())
    }

    fn unobserve_game(&self, game_id: GameId, listener_id: ListenerId) {
        self.spectator_service.unobserve_game(game_id, listener_id);
    }
}
