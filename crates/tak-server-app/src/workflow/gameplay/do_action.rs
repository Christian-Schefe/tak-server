use std::{sync::Arc, time::Instant};

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
        DoActionResult, Game, GamePlayerActionResult,
        request::{GameRequest, GameRequestType},
    },
};

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
        let now = Instant::now();
        let Ok(ctx) = self
            .ports
            .transaction_manager()
            .transaction()
            .await
            .map_err(|RepoError(e)| {
                tracing::error!("Failed to start transaction for game {}: {}", game_id, e);
            })
        else {
            return ActionResult::NotPossible(PlayerActionError::Internal);
        };
        let mut game = match self.ports.game_repository().get_game(&ctx, game_id).await {
            Ok(Some(Game::Ongoing(game))) => game,
            Ok(Some(Game::Finished(_))) => {
                return ActionResult::NotPossible(PlayerActionError::GameNotFound);
            }
            Ok(None) => return ActionResult::NotPossible(PlayerActionError::GameNotFound),
            Err(RepoError(e)) => {
                tracing::error!("Error retrieving game {}: {}", game_id, e);
                return ActionResult::NotPossible(PlayerActionError::Internal);
            }
        };
        let (maybe_action_record, maybe_ended_game) = match game.do_action(user_id, action, now) {
            GamePlayerActionResult::NotAPlayerInGame => {
                return ActionResult::NotPossible(PlayerActionError::NotAPlayerInGame);
            }
            GamePlayerActionResult::Result(DoActionResult::InvalidAction(e)) => {
                return ActionResult::ActionError(DoActionError::InvalidAction(e));
            }
            GamePlayerActionResult::Result(DoActionResult::NotPlayersTurn) => {
                return ActionResult::ActionError(DoActionError::NotPlayersTurn);
            }
            GamePlayerActionResult::Result(DoActionResult::ActionPerformed(action_record)) => {
                (Some(action_record), None)
            }
            GamePlayerActionResult::Result(DoActionResult::GameOver(ended_game, action_record)) => {
                (Some(action_record), Some(ended_game))
            }
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
                .notify_players_and_observers_of_game(game_id, &game.metadata, &msg)
                .await;
        }
        if let Some(ended_game) = maybe_ended_game {
            if let Err(()) = self
                .finalize_game_workflow
                .finalize_game(&ctx, game_id, game, ended_game)
                .await
            {
                tracing::error!("Failed to finalize game {}", game_id);
                return ActionResult::NotPossible(PlayerActionError::Internal);
            }
        }
        if let Err(RepoError(e)) = self.ports.transaction_manager().commit(ctx).await {
            tracing::error!("Failed to commit transaction for game {}: {}", game_id, e);
            return ActionResult::NotPossible(PlayerActionError::Internal);
        }

        ActionResult::Success
    }

    async fn set_request(
        &self,
        game_id: GameId,
        user_id: UserId,
        request: GameRequest,
    ) -> Result<(), PlayerActionError> {
        let now = Instant::now();
        let ctx = self.ports.transaction_manager().simple();
        let mut game = match self.ports.game_repository().get_game(&ctx, game_id).await {
            Ok(Some(Game::Ongoing(game))) => game,
            Ok(Some(Game::Finished(_))) => return Err(PlayerActionError::GameNotFound),
            Ok(None) => return Err(PlayerActionError::GameNotFound),
            Err(RepoError(e)) => {
                tracing::error!("Error retrieving game {}: {}", game_id, e);
                return Err(PlayerActionError::Internal);
            }
        };
        let maybe_finished_game = match game.set_request(user_id, request.clone(), now) {
            GamePlayerActionResult::NotAPlayerInGame => {
                return Err(PlayerActionError::NotAPlayerInGame);
            }
            GamePlayerActionResult::Timeout(ended_game) => Some(ended_game),
            GamePlayerActionResult::Result(None) => None,
            GamePlayerActionResult::Result(Some((request, time_info))) => {
                let msg = ListenerMessage::GameEvent {
                    game_id,
                    event_type: ListenerGameMessageType::GameRequestChanged { request },
                    time_info,
                };
                self.notify_player_workflow
                    .notify_players_and_observers_of_game(game_id, &game.metadata, &msg)
                    .await;
                None
            }
        };
        if let Some(ended_game) = maybe_finished_game {
            if let Err(()) = self
                .finalize_game_workflow
                .finalize_game(&ctx, game_id, game, ended_game)
                .await
            {
                tracing::error!("Failed to finalize game {}", game_id);
                return Err(PlayerActionError::Internal);
            }
        }
        Ok(())
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
        let now = Instant::now();
        let ctx = self.ports.transaction_manager().simple();
        let mut game = match self.ports.game_repository().get_game(&ctx, game_id).await {
            Ok(Some(Game::Ongoing(game))) => game,
            Ok(Some(Game::Finished(_))) => {
                return ActionResult::NotPossible(PlayerActionError::GameNotFound);
            }
            Ok(None) => return ActionResult::NotPossible(PlayerActionError::GameNotFound),
            Err(RepoError(e)) => {
                tracing::error!("Error retrieving game {}: {}", game_id, e);
                return ActionResult::NotPossible(PlayerActionError::Internal);
            }
        };
        let maybe_finished_game = match request_type {
            GameRequestType::Draw => match game.accept_draw_request(user_id, now) {
                GamePlayerActionResult::NotAPlayerInGame => {
                    return ActionResult::NotPossible(PlayerActionError::NotAPlayerInGame);
                }
                GamePlayerActionResult::Timeout(ended_game) => Some(ended_game),
                GamePlayerActionResult::Result(None) => {
                    return ActionResult::ActionError(HandleRequestError::RequestNotFound);
                }
                GamePlayerActionResult::Result(Some((request, time_info, ended_game))) => {
                    let request_msg = ListenerMessage::GameEvent {
                        game_id,
                        event_type: ListenerGameMessageType::GameRequestChanged { request },
                        time_info,
                    };
                    self.notify_player_workflow
                        .notify_players_and_observers_of_game(game_id, &game.metadata, &request_msg)
                        .await;
                    Some(ended_game)
                }
            },
            GameRequestType::Undo => match game.accept_undo_request(user_id, now) {
                GamePlayerActionResult::NotAPlayerInGame => {
                    return ActionResult::NotPossible(PlayerActionError::NotAPlayerInGame);
                }
                GamePlayerActionResult::Timeout(ended_game) => Some(ended_game),
                GamePlayerActionResult::Result(None) => {
                    return ActionResult::ActionError(HandleRequestError::RequestNotFound);
                }
                GamePlayerActionResult::Result(Some((request, time_info, undo_record))) => {
                    let request_msg = ListenerMessage::GameEvent {
                        game_id,
                        event_type: ListenerGameMessageType::GameRequestChanged { request },
                        time_info: time_info.clone(),
                    };
                    self.notify_player_workflow
                        .notify_players_and_observers_of_game(game_id, &game.metadata, &request_msg)
                        .await;
                    if let Some(undo_record) = undo_record {
                        let msg = ListenerMessage::GameEvent {
                            game_id,
                            event_type: ListenerGameMessageType::GameActionUndone {
                                ply_index: undo_record.ply_index,
                            },
                            time_info: time_info,
                        };
                        self.notify_player_workflow
                            .notify_players_and_observers_of_game(game_id, &game.metadata, &msg)
                            .await;
                    }
                    None
                }
            },
            GameRequestType::MoreTime => match game.accept_more_time_request(user_id, now) {
                GamePlayerActionResult::NotAPlayerInGame => {
                    return ActionResult::NotPossible(PlayerActionError::NotAPlayerInGame);
                }
                GamePlayerActionResult::Timeout(ended_game) => Some(ended_game),
                GamePlayerActionResult::Result(None) => {
                    return ActionResult::ActionError(HandleRequestError::RequestNotFound);
                }
                GamePlayerActionResult::Result(Some((request, time_info))) => {
                    let request_msg = ListenerMessage::GameEvent {
                        game_id,
                        event_type: ListenerGameMessageType::GameRequestChanged { request },
                        time_info,
                    };
                    self.notify_player_workflow
                        .notify_players_and_observers_of_game(game_id, &game.metadata, &request_msg)
                        .await;
                    None
                }
            },
        };
        if let Some(ended_game) = maybe_finished_game {
            if let Err(()) = self
                .finalize_game_workflow
                .finalize_game(&ctx, game_id, game, ended_game)
                .await
            {
                tracing::error!("Failed to finalize game {}", game_id);
                return ActionResult::NotPossible(PlayerActionError::Internal);
            }
        }
        ActionResult::Success
    }

    async fn resign(&self, game_id: GameId, user_id: UserId) -> Result<(), PlayerActionError> {
        let now = Instant::now();
        let ctx = self.ports.transaction_manager().simple();
        let mut game = match self.ports.game_repository().get_game(&ctx, game_id).await {
            Ok(Some(Game::Ongoing(game))) => game,
            Ok(Some(Game::Finished(_))) => return Err(PlayerActionError::GameNotFound),
            Ok(None) => return Err(PlayerActionError::GameNotFound),
            Err(RepoError(e)) => {
                tracing::error!("Error retrieving game {}: {}", game_id, e);
                return Err(PlayerActionError::Internal);
            }
        };
        let ended_game = match game.resign(user_id, now) {
            GamePlayerActionResult::NotAPlayerInGame => {
                return Err(PlayerActionError::NotAPlayerInGame);
            }
            GamePlayerActionResult::Timeout(ended_game) => ended_game,
            GamePlayerActionResult::Result(finished_game) => finished_game,
        };
        if let Err(()) = self
            .finalize_game_workflow
            .finalize_game(&ctx, game_id, game, ended_game)
            .await
        {
            tracing::error!("Failed to finalize game {}", game_id);
            return Err(PlayerActionError::Internal);
        }
        Ok(())
    }

    async fn abort(&self, game_id: GameId, user_id: UserId) -> ActionResult<AbortError> {
        let now = Instant::now();
        let ctx = self.ports.transaction_manager().simple();
        let mut game = match self.ports.game_repository().get_game(&ctx, game_id).await {
            Ok(Some(Game::Ongoing(game))) => game,
            Ok(Some(Game::Finished(_))) => {
                return ActionResult::NotPossible(PlayerActionError::GameNotFound);
            }
            Ok(None) => return ActionResult::NotPossible(PlayerActionError::GameNotFound),
            Err(RepoError(e)) => {
                tracing::error!("Error retrieving game {}: {}", game_id, e);
                return ActionResult::NotPossible(PlayerActionError::Internal);
            }
        };
        match game.try_abort(user_id, now) {
            GamePlayerActionResult::NotAPlayerInGame => {
                ActionResult::NotPossible(PlayerActionError::NotAPlayerInGame)
            }
            GamePlayerActionResult::Timeout(ended_game) => {
                if let Err(()) = self
                    .finalize_game_workflow
                    .finalize_game(&ctx, game_id, game, ended_game)
                    .await
                {
                    tracing::error!("Failed to finalize game {}", game_id);
                    return ActionResult::NotPossible(PlayerActionError::Internal);
                }
                ActionResult::NotPossible(PlayerActionError::GameNotFound)
            }
            GamePlayerActionResult::Result(Some(finished_game)) => {
                if let Err(()) = self
                    .finalize_game_workflow
                    .finalize_game(&ctx, game_id, game, finished_game)
                    .await
                {
                    tracing::error!("Failed to finalize game {}", game_id);
                    return ActionResult::NotPossible(PlayerActionError::Internal);
                }
                ActionResult::Success
            }
            GamePlayerActionResult::Result(None) => {
                ActionResult::ActionError(AbortError::GameAlreadyStarted)
            }
        }
    }
}
