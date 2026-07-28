use std::{sync::Arc, time::Instant};

use tak_core::TakAction;

use crate::{
    domain::{
        GameId, PlayerId,
        game::{
            DoActionResult, FinishedGame, GamePlayerActionResult, GameService,
            request::{GameRequest, GameRequestType},
        },
    },
    ports::notification::{ListenerGameMessageType, ListenerMessage},
    workflow::{
        gameplay::finalize_game::FinalizeGameWorkflow, player::notify_player::NotifyPlayerWorkflow,
    },
};

#[async_trait::async_trait]
pub trait DoActionUseCase {
    async fn do_action(
        &self,
        game_id: GameId,
        player_id: PlayerId,
        action: TakAction,
    ) -> ActionResult<DoActionError>;
    async fn set_request(
        &self,
        game_id: GameId,
        player_id: PlayerId,
        request: GameRequest,
    ) -> Result<(), PlayerActionError>;
    async fn accept_request(
        &self,
        game_id: GameId,
        player_id: PlayerId,
        request_type: GameRequestType,
    ) -> ActionResult<HandleRequestError>;
    async fn resign(&self, game_id: GameId, player_id: PlayerId) -> Result<(), PlayerActionError>;
    async fn abort(&self, game_id: GameId, player_id: PlayerId) -> ActionResult<AbortError>;
}

#[derive(Debug)]
pub enum PlayerActionError {
    GameNotFound,
    NotAPlayerInGame,
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

pub struct DoActionUseCaseImpl<G: GameService, NP: NotifyPlayerWorkflow, F: FinalizeGameWorkflow> {
    game_service: Arc<G>,
    notify_player_workflow: Arc<NP>,
    finalize_game_workflow: Arc<F>,
}

impl<G: GameService, NP: NotifyPlayerWorkflow, F: FinalizeGameWorkflow>
    DoActionUseCaseImpl<G, NP, F>
{
    pub fn new(
        game_service: Arc<G>,
        notify_player_workflow: Arc<NP>,
        finalize_game_workflow: Arc<F>,
    ) -> Self {
        Self {
            game_service,
            notify_player_workflow,
            finalize_game_workflow,
        }
    }

    async fn handle_ended_game(&self, ended_game: FinishedGame) {
        //self.send_game_time_update_for_finished_game(&ended_game)
        //    .await;
        self.finalize_game_workflow.finalize_game(ended_game).await;
    }

    async fn handle_game_action_result<R>(
        &self,
        result: GamePlayerActionResult<R>,
    ) -> Result<R, PlayerActionError> {
        match result {
            GamePlayerActionResult::Result(res) => Ok(res),
            GamePlayerActionResult::Timeout(ended_game) => {
                self.handle_ended_game(ended_game).await;
                Err(PlayerActionError::GameNotFound)
            }
            GamePlayerActionResult::GameNotFound => Err(PlayerActionError::GameNotFound),
            GamePlayerActionResult::NotAPlayerInGame => Err(PlayerActionError::NotAPlayerInGame),
        }
    }
}

#[async_trait::async_trait]
impl<
    G: GameService + Send + Sync + 'static,
    NP: NotifyPlayerWorkflow + Send + Sync + 'static,
    F: FinalizeGameWorkflow + Send + Sync + 'static,
> DoActionUseCase for DoActionUseCaseImpl<G, NP, F>
{
    async fn do_action(
        &self,
        game_id: GameId,
        player_id: PlayerId,
        action: TakAction,
    ) -> ActionResult<DoActionError> {
        tracing::debug!(
            "Player {} is performing action {:?} in game {}",
            player_id,
            action,
            game_id
        );
        let now = Instant::now();
        let (action_record, maybe_ended_game) = match self
            .handle_game_action_result(self.game_service.do_action(game_id, player_id, action, now))
            .await
        {
            Err(e) => return ActionResult::NotPossible(e),
            Ok(DoActionResult::ActionPerformed(action_record)) => (action_record, None),
            Ok(DoActionResult::GameOver(action_record, ended_game)) => {
                (action_record, Some(ended_game))
            }
            Ok(DoActionResult::InvalidAction(e)) => {
                return ActionResult::ActionError(DoActionError::InvalidAction(e));
            }
            Ok(DoActionResult::NotPlayersTurn) => {
                return ActionResult::ActionError(DoActionError::NotPlayersTurn);
            }
        };

        let msg = ListenerMessage::GameEvent {
            game_id,
            event_type: ListenerGameMessageType::GameAction {
                player_id,
                action: action_record.action,
                ply_index: action_record.ply_index,
            },
            time_info: action_record.time_info,
        };

        // Needs different notification flow as game domain removes game once ended
        if let Some(ended_game) = maybe_ended_game {
            self.notify_player_workflow
                .notify_players_and_observers_of_game(
                    ended_game.game_id,
                    &ended_game.metadata,
                    &msg,
                )
                .await;

            self.handle_ended_game(ended_game).await;
        } else {
            self.notify_player_workflow
                .notify_players_and_observers(game_id, &msg)
                .await;
        }

        ActionResult::Success
    }

    async fn set_request(
        &self,
        game_id: GameId,
        player_id: PlayerId,
        request: GameRequest,
    ) -> Result<(), PlayerActionError> {
        let now = Instant::now();
        match self
            .handle_game_action_result(self.game_service.set_request(
                game_id,
                player_id,
                request.clone(),
                now,
            ))
            .await
        {
            Err(e) => Err(e),
            Ok(None) => Ok(()),
            Ok(Some((request, time_info))) => {
                let msg = ListenerMessage::GameEvent {
                    game_id,
                    event_type: ListenerGameMessageType::GameRequestChanged { request },
                    time_info,
                };
                self.notify_player_workflow
                    .notify_players_and_observers(game_id, &msg)
                    .await;
                Ok(())
            }
        }
    }

    async fn accept_request(
        &self,
        game_id: GameId,
        player_id: PlayerId,
        request_type: GameRequestType,
    ) -> ActionResult<HandleRequestError> {
        tracing::info!(
            "Player {} is accepting request of type {:?} in game {}",
            player_id,
            request_type,
            game_id
        );
        let now = Instant::now();
        match request_type {
            GameRequestType::Draw => {
                match self
                    .handle_game_action_result(
                        self.game_service
                            .accept_draw_request(game_id, player_id, now),
                    )
                    .await
                {
                    Err(e) => ActionResult::NotPossible(e),
                    Ok(Some((request, time_info, ended_game))) => {
                        let request_msg = ListenerMessage::GameEvent {
                            game_id,
                            event_type: ListenerGameMessageType::GameRequestChanged { request },
                            time_info,
                        };
                        self.notify_player_workflow
                            .notify_players_and_observers_of_game(
                                ended_game.game_id,
                                &ended_game.metadata,
                                &request_msg,
                            )
                            .await;
                        self.handle_ended_game(ended_game).await;
                        ActionResult::Success
                    }
                    Ok(None) => ActionResult::ActionError(HandleRequestError::RequestNotFound),
                }
            }
            GameRequestType::Undo => {
                match self
                    .handle_game_action_result(
                        self.game_service
                            .accept_undo_request(game_id, player_id, now),
                    )
                    .await
                {
                    Err(e) => ActionResult::NotPossible(e),
                    Ok(Some((request, time_info, undo_record))) => {
                        let request_msg = ListenerMessage::GameEvent {
                            game_id,
                            event_type: ListenerGameMessageType::GameRequestChanged { request },
                            time_info: time_info.clone(),
                        };
                        self.notify_player_workflow
                            .notify_players_and_observers(game_id, &request_msg)
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
                                .notify_players_and_observers(game_id, &msg)
                                .await;
                        }
                        ActionResult::Success
                    }
                    Ok(None) => ActionResult::ActionError(HandleRequestError::RequestNotFound),
                }
            }
            GameRequestType::MoreTime => {
                match self
                    .handle_game_action_result(
                        self.game_service
                            .accept_more_time_request(game_id, player_id, now),
                    )
                    .await
                {
                    Err(e) => ActionResult::NotPossible(e),
                    Ok(Some((request, time_info))) => {
                        let request_msg = ListenerMessage::GameEvent {
                            game_id,
                            event_type: ListenerGameMessageType::GameRequestChanged { request },
                            time_info,
                        };
                        self.notify_player_workflow
                            .notify_players_and_observers(game_id, &request_msg)
                            .await;
                        ActionResult::Success
                    }
                    Ok(None) => ActionResult::ActionError(HandleRequestError::RequestNotFound),
                }
            }
        }
    }

    async fn resign(&self, game_id: GameId, player_id: PlayerId) -> Result<(), PlayerActionError> {
        let now = Instant::now();
        let ended_game = self
            .handle_game_action_result(self.game_service.resign(game_id, player_id, now))
            .await?;
        self.handle_ended_game(ended_game).await;
        Ok(())
    }

    async fn abort(&self, game_id: GameId, player_id: PlayerId) -> ActionResult<AbortError> {
        let now = Instant::now();
        match self
            .handle_game_action_result(self.game_service.abort(game_id, player_id, now))
            .await
        {
            Ok(Some(ended_game)) => {
                self.handle_ended_game(ended_game).await;
                ActionResult::Success
            }
            Ok(None) => ActionResult::ActionError(AbortError::GameAlreadyStarted),
            Err(e) => ActionResult::NotPossible(e),
        }
    }
}
