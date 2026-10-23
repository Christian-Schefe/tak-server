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
        gameplay::lifecycle::LifecycleGameWorkflow, player::notify_player::NotifyPlayerWorkflow,
    },
};
use tak_core::TakAction;
use tak_domain::{
    GameId, UserId,
    game::{
        CheckDisconnectTimeoutResult, DoActionResult, GameOverResult,
        request::{GameRequest, GameRequestType},
    },
};
pub enum ObserveOutcome {
    Finished,
    Continue(std::time::Duration),
}

pub enum ObserveError {
    GameNotFound,
    Internal,
}

#[async_trait::async_trait]
pub trait DoActionUseCase {
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

    async fn check_game_timeout(&self, game_id: GameId) -> Result<ObserveOutcome, ObserveError>;
    async fn check_player_timeout(
        &self,
        user_id: UserId,
        disconnected_at: Instant,
    ) -> Result<ObserveOutcome, ObserveError>;
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

pub struct DoActionUseCaseImpl<
    Ports: PortsCollection,
    NP: NotifyPlayerWorkflow,
    F: LifecycleGameWorkflow<Ports::TrCtx>,
> {
    ports: Arc<Ports>,
    notify_player_workflow: Arc<NP>,
    finalize_game_workflow: Arc<F>,
}

impl<Ports: PortsCollection, NP: NotifyPlayerWorkflow, F: LifecycleGameWorkflow<Ports::TrCtx>>
    DoActionUseCaseImpl<Ports, NP, F>
{
    pub fn new(
        ports: Arc<Ports>,
        notify_player_workflow: Arc<NP>,
        finalize_game_workflow: Arc<F>,
    ) -> Self {
        Self {
            ports,
            notify_player_workflow,
            finalize_game_workflow,
        }
    }
}

#[async_trait::async_trait]
impl<
    Ports: PortsCollection + Send + Sync + 'static,
    NP: NotifyPlayerWorkflow + Send + Sync + 'static,
    F: LifecycleGameWorkflow<Ports::TrCtx> + Send + Sync + 'static,
> DoActionUseCase for DoActionUseCaseImpl<Ports, NP, F>
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
        let mut message = None;
        match self
            .finalize_game_workflow
            .modify_game(game_id, |game, metadata| {
                let Some(player) = metadata.get_player(user_id) else {
                    return Err(ActionError::NotPossible(
                        PlayerActionError::NotAPlayerInGame,
                    ));
                };
                match game.do_action(player, action, now) {
                    DoActionResult::InvalidAction(e) => {
                        return Err(ActionError::ActionError(DoActionError::InvalidAction(e)));
                    }
                    DoActionResult::NotPlayersTurn => {
                        return Err(ActionError::ActionError(DoActionError::NotPlayersTurn));
                    }
                    DoActionResult::ActionPerformed(action_record) => {
                        message = Some((
                            ListenerMessage::GameEvent {
                                game_id,
                                event_type: ListenerGameMessageType::GameAction {
                                    user_id,
                                    action: action_record.action,
                                    ply_index: action_record.ply_index,
                                },
                                time_info: action_record.time_info,
                            },
                            metadata.white.user_id,
                            metadata.black.user_id,
                        ));
                    }
                    DoActionResult::GameOver => {}
                };

                Ok(())
            })
            .await
        {
            Ok(Err(e)) => return Err(e),
            Ok(Ok(())) => {}
            Err(()) => return Err(ActionError::NotPossible(PlayerActionError::Internal)),
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
                    GameOverResult::GameOver => Ok(()),
                    GameOverResult::Result(false) => Ok(()),
                    GameOverResult::Result(true) => {
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
                        GameOverResult::GameOver => Ok(()),
                        GameOverResult::Result(false) => {
                            return Err(ActionError::ActionError(
                                HandleRequestError::RequestNotFound,
                            ));
                        }
                        GameOverResult::Result(true) => {
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
                        GameOverResult::GameOver => Ok(()),
                        GameOverResult::Result(None) => {
                            return Err(ActionError::ActionError(
                                HandleRequestError::RequestNotFound,
                            ));
                        }
                        GameOverResult::Result(Some(undo_record)) => {
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
                            if let Some(undo_record) = undo_record {
                                messages.push((
                                    ListenerMessage::GameEvent {
                                        game_id,
                                        event_type: ListenerGameMessageType::GameActionUndone {
                                            ply_index: undo_record.ply_index,
                                        },
                                        time_info: time_info.clone(),
                                    },
                                    metadata.white.user_id,
                                    metadata.black.user_id,
                                ));
                            }
                            Ok(())
                        }
                    },
                    GameRequestType::MoreTime => match game.accept_more_time_request(player, now) {
                        GameOverResult::GameOver => Ok(()),
                        GameOverResult::Result(false) => {
                            return Err(ActionError::ActionError(
                                HandleRequestError::RequestNotFound,
                            ));
                        }
                        GameOverResult::Result(true) => {
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
                    GameOverResult::GameOver => Ok(()),
                    GameOverResult::Result(()) => Ok(()),
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
                match game.try_abort(now) {
                    GameOverResult::GameOver => Ok(()),
                    GameOverResult::Result(false) => {
                        return Err(ActionError::ActionError(AbortError::GameAlreadyStarted));
                    }
                    GameOverResult::Result(true) => Ok(()),
                }
            })
            .await
        {
            Ok(Err(e)) => return Err(e),
            Ok(Ok(())) => Ok(()),
            Err(()) => return Err(ActionError::NotPossible(PlayerActionError::Internal)),
        }
    }

    async fn check_game_timeout(&self, game_id: GameId) -> Result<ObserveOutcome, ObserveError> {
        let mut result = ObserveOutcome::Finished;
        match self
            .finalize_game_workflow
            .modify_game(game_id, |game, _| {
                let now = SystemTime::now();
                match game.check_timeout(now) {
                    GameOverResult::GameOver => Ok(()),
                    GameOverResult::Result(time_info) => {
                        result = ObserveOutcome::Continue(
                            time_info.white_remaining.min(time_info.black_remaining)
                                + std::time::Duration::from_millis(100),
                        );
                        Ok(())
                    }
                }
            })
            .await
        {
            Ok(Ok(())) => {}
            Ok(Err(())) => return Err(ObserveError::GameNotFound),
            Err(()) => return Err(ObserveError::Internal),
        };
        Ok(result)
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
                        GameOverResult::GameOver => Ok(()),
                        GameOverResult::Result(res) => match res {
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
}
