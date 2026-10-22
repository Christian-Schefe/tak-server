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
        gameplay::finalize_game::FinalizeGameWorkflow, player::notify_player::NotifyPlayerWorkflow,
    },
};
use tak_core::TakAction;
use tak_domain::{
    GameId, UserId,
    game::{
        CheckDisconnectTimeoutResult, CheckTimeoutResult, DoActionResult, GamePlayerActionResult,
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
    ) -> ActionResult<DoActionError>;
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
    ) -> ActionResult<HandleRequestError>;
    async fn resign(&self, game_id: GameId, user_id: UserId) -> Result<(), PlayerActionError>;
    async fn abort(&self, game_id: GameId, user_id: UserId) -> ActionResult<AbortError>;

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
pub enum ActionResult<R> {
    Success,
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
    F: FinalizeGameWorkflow<Ports::TrCtx>,
> {
    ports: Arc<Ports>,
    notify_player_workflow: Arc<NP>,
    finalize_game_workflow: Arc<F>,
}

impl<Ports: PortsCollection, NP: NotifyPlayerWorkflow, F: FinalizeGameWorkflow<Ports::TrCtx>>
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
    F: FinalizeGameWorkflow<Ports::TrCtx> + Send + Sync + 'static,
> DoActionUseCase for DoActionUseCaseImpl<Ports, NP, F>
{
    async fn do_action(
        &self,
        game_id: GameId,
        user_id: UserId,
        action: TakAction,
    ) -> ActionResult<DoActionError> {
        tracing::debug!(
            "Player {} is performing action {:?} in game {}",
            user_id,
            action,
            game_id
        );
        match self
            .finalize_game_workflow
            .modify_game(game_id, |game, metadata| {
                let now = SystemTime::now();
                let Some(player) = metadata.get_player(user_id) else {
                    return Err(ActionResult::NotPossible(
                        PlayerActionError::NotAPlayerInGame,
                    ));
                };
                let (maybe_action_record, maybe_ended_game) =
                    match game.do_action(player, action, now) {
                        GamePlayerActionResult::Result(DoActionResult::InvalidAction(e)) => {
                            return Err(ActionResult::ActionError(DoActionError::InvalidAction(e)));
                        }
                        GamePlayerActionResult::Result(DoActionResult::NotPlayersTurn) => {
                            return Err(ActionResult::ActionError(DoActionError::NotPlayersTurn));
                        }
                        GamePlayerActionResult::Result(DoActionResult::ActionPerformed(
                            action_record,
                        )) => (Some(action_record), None),
                        GamePlayerActionResult::Result(DoActionResult::GameOver(
                            ended_game,
                            action_record,
                        )) => (Some(action_record), Some(ended_game)),
                        GamePlayerActionResult::Timeout(ended_game) => (None, Some(ended_game)),
                    };
                if let Some(action_record) = maybe_action_record {
                    let msg = ListenerMessage::GameEvent {
                        game_id,
                        event_type: ListenerGameMessageType::GameAction {
                            user_id,
                            action: action_record.action,
                            ply_index: action_record.ply_index,
                        },
                        time_info: action_record.time_info,
                    };
                    self.notify_player_workflow
                        .notify_players_and_observers_of_game(game_id, &metadata, &msg);
                }
                Ok(maybe_ended_game)
            })
            .await
        {
            Ok(Err(e)) => return e,
            Ok(Ok(())) => ActionResult::Success,
            Err(()) => return ActionResult::NotPossible(PlayerActionError::Internal),
        }
    }

    async fn set_request(
        &self,
        game_id: GameId,
        user_id: UserId,
        request: GameRequest,
    ) -> Result<(), PlayerActionError> {
        let now = SystemTime::now();

        match self
            .finalize_game_workflow
            .modify_game(game_id, |game, metadata| {
                let Some(player) = metadata.get_player(user_id) else {
                    return Err(PlayerActionError::NotAPlayerInGame);
                };
                match game.set_request(player, request.clone(), now) {
                    GamePlayerActionResult::Timeout(ended_game) => Ok(Some(ended_game)),
                    GamePlayerActionResult::Result(None) => Ok(None),
                    GamePlayerActionResult::Result(Some((request, time_info))) => {
                        let msg = ListenerMessage::GameEvent {
                            game_id,
                            event_type: ListenerGameMessageType::GameRequestChanged {
                                user_id,
                                request,
                            },
                            time_info,
                        };
                        self.notify_player_workflow
                            .notify_players_and_observers_of_game(game_id, &metadata, &msg);
                        Ok(None)
                    }
                }
            })
            .await
        {
            Ok(res) => res,
            Err(()) => return Err(PlayerActionError::Internal),
        }
    }

    async fn accept_request(
        &self,
        game_id: GameId,
        user_id: UserId,
        request_type: GameRequestType,
    ) -> ActionResult<HandleRequestError> {
        tracing::info!(
            "Player {} is accepting request of type {:?} in game {}",
            user_id,
            request_type,
            game_id
        );
        let now = SystemTime::now();
        match self
            .finalize_game_workflow
            .modify_game(game_id, |game, metadata| {
                let Some(player) = metadata.get_player(user_id) else {
                    return Err(ActionResult::NotPossible(
                        PlayerActionError::NotAPlayerInGame,
                    ));
                };
                let opponent_id = metadata.get_user_id(player.opponent());
                match request_type {
                    GameRequestType::Draw => match game.accept_draw_request(player, now) {
                        GamePlayerActionResult::Timeout(ended_game) => Ok(Some(ended_game)),
                        GamePlayerActionResult::Result(None) => {
                            return Err(ActionResult::ActionError(
                                HandleRequestError::RequestNotFound,
                            ));
                        }
                        GamePlayerActionResult::Result(Some((request, ended_game))) => {
                            let request_msg = ListenerMessage::GameEvent {
                                game_id,
                                event_type: ListenerGameMessageType::GameRequestChanged {
                                    user_id: opponent_id,
                                    request,
                                },
                                time_info: ended_game.time_info.clone(),
                            };
                            self.notify_player_workflow
                                .notify_players_and_observers_of_game(
                                    game_id,
                                    &metadata,
                                    &request_msg,
                                );
                            Ok(Some(ended_game))
                        }
                    },
                    GameRequestType::Undo => match game.accept_undo_request(player, now) {
                        GamePlayerActionResult::Timeout(ended_game) => Ok(Some(ended_game)),
                        GamePlayerActionResult::Result(None) => {
                            return Err(ActionResult::ActionError(
                                HandleRequestError::RequestNotFound,
                            ));
                        }
                        GamePlayerActionResult::Result(Some((request, time_info, undo_record))) => {
                            let request_msg = ListenerMessage::GameEvent {
                                game_id,
                                event_type: ListenerGameMessageType::GameRequestChanged {
                                    user_id: opponent_id,
                                    request,
                                },
                                time_info: time_info.clone(),
                            };
                            self.notify_player_workflow
                                .notify_players_and_observers_of_game(
                                    game_id,
                                    &metadata,
                                    &request_msg,
                                );
                            if let Some(undo_record) = undo_record {
                                let msg = ListenerMessage::GameEvent {
                                    game_id,
                                    event_type: ListenerGameMessageType::GameActionUndone {
                                        ply_index: undo_record.ply_index,
                                    },
                                    time_info,
                                };
                                self.notify_player_workflow
                                    .notify_players_and_observers_of_game(game_id, &metadata, &msg);
                            }
                            Ok(None)
                        }
                    },
                    GameRequestType::MoreTime => match game.accept_more_time_request(player, now) {
                        GamePlayerActionResult::Timeout(ended_game) => Ok(Some(ended_game)),
                        GamePlayerActionResult::Result(None) => {
                            return Err(ActionResult::ActionError(
                                HandleRequestError::RequestNotFound,
                            ));
                        }
                        GamePlayerActionResult::Result(Some((request, time_info))) => {
                            let request_msg = ListenerMessage::GameEvent {
                                game_id,
                                event_type: ListenerGameMessageType::GameRequestChanged {
                                    user_id: opponent_id,
                                    request,
                                },
                                time_info,
                            };
                            self.notify_player_workflow
                                .notify_players_and_observers_of_game(
                                    game_id,
                                    &metadata,
                                    &request_msg,
                                );
                            Ok(None)
                        }
                    },
                }
            })
            .await
        {
            Ok(Err(e)) => e,
            Ok(Ok(())) => ActionResult::Success,
            Err(()) => return ActionResult::NotPossible(PlayerActionError::Internal),
        }
    }

    async fn resign(&self, game_id: GameId, user_id: UserId) -> Result<(), PlayerActionError> {
        let now = SystemTime::now();
        match self
            .finalize_game_workflow
            .modify_game(game_id, |game, metadata| {
                let Some(player) = metadata.get_player(user_id) else {
                    return Err(PlayerActionError::NotAPlayerInGame);
                };
                let ended_game = match game.resign(player, now) {
                    GamePlayerActionResult::Timeout(ended_game) => ended_game,
                    GamePlayerActionResult::Result(finished_game) => finished_game,
                };
                Ok(Some(ended_game))
            })
            .await
        {
            Ok(res) => res,
            Err(()) => return Err(PlayerActionError::Internal),
        }
    }

    async fn abort(&self, game_id: GameId, user_id: UserId) -> ActionResult<AbortError> {
        let now = SystemTime::now();
        match self
            .finalize_game_workflow
            .modify_game(game_id, |game, metadata| {
                let Some(_) = metadata.get_player(user_id) else {
                    return Err(ActionResult::NotPossible(
                        PlayerActionError::NotAPlayerInGame,
                    ));
                };
                match game.try_abort(now) {
                    GamePlayerActionResult::Timeout(ended_game) => Ok(Some(ended_game)),
                    GamePlayerActionResult::Result(None) => {
                        return Err(ActionResult::ActionError(AbortError::GameAlreadyStarted));
                    }
                    GamePlayerActionResult::Result(Some(ended_game)) => Ok(Some(ended_game)),
                }
            })
            .await
        {
            Ok(Err(e)) => return e,
            Ok(Ok(())) => ActionResult::Success,
            Err(()) => return ActionResult::NotPossible(PlayerActionError::Internal),
        }
    }

    async fn check_game_timeout(&self, game_id: GameId) -> Result<ObserveOutcome, ObserveError> {
        let mut result = ObserveOutcome::Finished;
        match self
            .finalize_game_workflow
            .modify_game(game_id, |game, _| {
                let now = SystemTime::now();
                match game.check_timeout(now) {
                    CheckTimeoutResult::TimedOut(finished_game) => {
                        tracing::info!("Game {:?} timed out, finalizing game", game_id);
                        Ok(Some(finished_game))
                    }
                    CheckTimeoutResult::NoTimeout(time_info) => {
                        result = ObserveOutcome::Continue(
                            time_info.white_remaining.min(time_info.black_remaining)
                                + std::time::Duration::from_millis(100),
                        );
                        Ok(None)
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
                        return Ok(None);
                    };
                    match game.check_disconnect_timeout(player, disconnected_duration, now) {
                        GamePlayerActionResult::Timeout(ended_game) => Ok(Some(ended_game)),
                        GamePlayerActionResult::Result(res) => match res {
                            CheckDisconnectTimeoutResult::TimedOut(ended_game) => {
                                tracing::info!(
                                    "Player {:?} timed out in game {:?} after {:?}",
                                    user_id,
                                    game_id,
                                    disconnected_duration
                                );
                                Ok(Some(ended_game))
                            }
                            CheckDisconnectTimeoutResult::CantTimeOut => Ok(None),
                            CheckDisconnectTimeoutResult::NoTimeout(duration) => {
                                wait_duration = match wait_duration {
                                    Some(current) if current < duration => Some(current),
                                    _ => Some(duration),
                                };
                                Ok(None)
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
