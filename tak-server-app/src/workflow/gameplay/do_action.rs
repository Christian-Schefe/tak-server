use std::{sync::Arc, time::Instant};

use tak_core::{TakAction, TakRequest, TakRequestId, TakRequestType};

use crate::{
    domain::{
        GameId, PlayerId,
        game::{DoActionResult, FinishedGame, GamePlayerActionResult, GameService, ResignResult},
    },
    ports::notification::ListenerMessage,
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
    fn get_request(&self, game_id: GameId, request_id: TakRequestId) -> Option<TakRequest>;
    fn get_requests_of_player(
        &self,
        game_id: GameId,
        player_id: PlayerId,
    ) -> Option<Vec<TakRequest>>;
    async fn add_request(
        &self,
        game_id: GameId,
        player_id: PlayerId,
        request_type: TakRequestType,
    ) -> ActionResult<AddRequestError>;
    async fn retract_request(
        &self,
        game_id: GameId,
        player_id: PlayerId,
        request_id: TakRequestId,
    ) -> ActionResult<HandleRequestError>;
    async fn reject_request(
        &self,
        game_id: GameId,
        player_id: PlayerId,
        request_id: TakRequestId,
    ) -> ActionResult<HandleRequestError>;
    async fn accept_draw_request(
        &self,
        game_id: GameId,
        player_id: PlayerId,
        request_id: TakRequestId,
    ) -> ActionResult<HandleRequestError>;
    async fn accept_undo_request(
        &self,
        game_id: GameId,
        player_id: PlayerId,
        request_id: TakRequestId,
    ) -> ActionResult<HandleRequestError>;
    async fn resign(&self, game_id: GameId, player_id: PlayerId) -> Result<(), PlayerActionError>;
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
pub enum AddRequestError {
    AlreadyRequested,
}

#[derive(Debug)]
pub enum HandleRequestError {
    RequestNotFound,
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

    async fn send_game_time_update(&self, game_id: GameId, now: Instant) {
        if let Some(game) = self.game_service.get_game_by_id(game_id) {
            let time_remaining = game.get_time_remaining(now);
            let time_update_msg = ListenerMessage::GameTimeUpdate {
                game_id,
                white_time: time_remaining.white_time,
                black_time: time_remaining.black_time,
            };
            self.notify_player_workflow
                .notify_players_and_observers_of_game(&game.metadata, &time_update_msg)
                .await;
        }
    }

    async fn send_game_time_update_for_finished_game(&self, game: &FinishedGame) {
        let time_remaining = game.get_time_remaining();
        let time_update_msg = ListenerMessage::GameTimeUpdate {
            game_id: game.metadata.game_id,
            white_time: time_remaining.white_time,
            black_time: time_remaining.black_time,
        };
        self.notify_player_workflow
            .notify_players_and_observers_of_game(&game.metadata, &time_update_msg)
            .await;
    }

    async fn handle_ended_game(&self, ended_game: FinishedGame) {
        self.send_game_time_update_for_finished_game(&ended_game)
            .await;
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
        log::debug!(
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

        let msg = ListenerMessage::GameAction {
            game_id,
            player_id,
            action: action_record,
        };

        // Needs different notification flow as game domain removes game once ended
        if let Some(ended_game) = maybe_ended_game {
            self.notify_player_workflow
                .notify_players_and_observers_of_game(&ended_game.metadata, &msg)
                .await;

            self.handle_ended_game(ended_game).await;
        } else {
            self.notify_player_workflow
                .notify_players_and_observers(game_id, &msg)
                .await;

            self.send_game_time_update(game_id, now).await;
        }

        ActionResult::Success
    }

    fn get_request(&self, game_id: GameId, request_id: TakRequestId) -> Option<TakRequest> {
        self.game_service.get_request(game_id, request_id)
    }

    fn get_requests_of_player(
        &self,
        game_id: GameId,
        player_id: PlayerId,
    ) -> Option<Vec<TakRequest>> {
        self.game_service.get_requests_of_player(game_id, player_id)
    }

    async fn add_request(
        &self,
        game_id: GameId,
        player_id: PlayerId,
        request_type: TakRequestType,
    ) -> ActionResult<AddRequestError> {
        let now = Instant::now();
        match self
            .handle_game_action_result(self.game_service.add_request(
                game_id,
                player_id,
                request_type,
                now,
            ))
            .await
        {
            Err(e) => return ActionResult::NotPossible(e),
            Ok(Err(())) => {
                return ActionResult::ActionError(AddRequestError::AlreadyRequested);
            }
            Ok(Ok(request)) => {
                let msg = ListenerMessage::GameRequestAdded {
                    game_id,
                    requesting_player_id: player_id,
                    request,
                };
                self.notify_player_workflow
                    .notify_players_and_observers(game_id, &msg)
                    .await;
                ActionResult::Success
            }
        }
    }

    async fn retract_request(
        &self,
        game_id: GameId,
        player_id: PlayerId,
        request_id: TakRequestId,
    ) -> ActionResult<HandleRequestError> {
        let now = Instant::now();
        match self
            .handle_game_action_result(
                self.game_service
                    .retract_request(game_id, player_id, request_id, now),
            )
            .await
        {
            Err(e) => return ActionResult::NotPossible(e),
            Ok(Ok(request)) => {
                let msg = ListenerMessage::GameRequestRetracted {
                    game_id,
                    retracting_player_id: player_id,
                    request,
                };
                self.notify_player_workflow
                    .notify_players_and_observers(game_id, &msg)
                    .await;
                ActionResult::Success
            }
            Ok(Err(())) => ActionResult::ActionError(HandleRequestError::RequestNotFound),
        }
    }
    async fn reject_request(
        &self,
        game_id: GameId,
        player_id: PlayerId,
        request_id: TakRequestId,
    ) -> ActionResult<HandleRequestError> {
        log::info!(
            "Player {} is rejecting request {:?} in game {}",
            player_id,
            request_id,
            game_id
        );
        let now = Instant::now();
        match self
            .handle_game_action_result(
                self.game_service
                    .reject_request(game_id, player_id, request_id, now),
            )
            .await
        {
            Err(e) => return ActionResult::NotPossible(e),
            Ok(Ok(request)) => {
                let msg = ListenerMessage::GameRequestRejected {
                    game_id,
                    rejecting_player_id: player_id,
                    request,
                };
                self.notify_player_workflow
                    .notify_players_and_observers(game_id, &msg)
                    .await;
                ActionResult::Success
            }
            Ok(Err(())) => ActionResult::ActionError(HandleRequestError::RequestNotFound),
        }
    }

    async fn accept_draw_request(
        &self,
        game_id: GameId,
        player_id: PlayerId,
        request_id: TakRequestId,
    ) -> ActionResult<HandleRequestError> {
        log::info!(
            "Player {} is accepting draw request {:?} in game {}",
            player_id,
            request_id,
            game_id
        );
        let now = Instant::now();
        match self
            .handle_game_action_result(
                self.game_service
                    .accept_draw_request(game_id, player_id, request_id, now),
            )
            .await
        {
            Err(e) => ActionResult::NotPossible(e),
            Ok(Ok((request, ended_game))) => {
                let request_msg = ListenerMessage::GameRequestAccepted {
                    game_id,
                    accepting_player_id: player_id,
                    request,
                };
                self.notify_player_workflow
                    .notify_players_and_observers_of_game(&ended_game.metadata, &request_msg)
                    .await;
                self.handle_ended_game(ended_game).await;
                ActionResult::Success
            }
            Ok(Err(())) => ActionResult::ActionError(HandleRequestError::RequestNotFound),
        }
    }

    async fn accept_undo_request(
        &self,
        game_id: GameId,
        player_id: PlayerId,
        request_id: TakRequestId,
    ) -> ActionResult<HandleRequestError> {
        log::info!(
            "Player {} is accepting undo request {:?} in game {}",
            player_id,
            request_id,
            game_id
        );
        let now = Instant::now();
        match self
            .handle_game_action_result(
                self.game_service
                    .accept_undo_request(game_id, player_id, request_id, now),
            )
            .await
        {
            Err(e) => ActionResult::NotPossible(e),
            Ok(Ok(request)) => {
                let request_msg = ListenerMessage::GameRequestAccepted {
                    game_id,
                    accepting_player_id: player_id,
                    request,
                };
                self.notify_player_workflow
                    .notify_players_and_observers(game_id, &request_msg)
                    .await;
                let msg = ListenerMessage::GameActionUndone { game_id };
                self.notify_player_workflow
                    .notify_players_and_observers(game_id, &msg)
                    .await;
                ActionResult::Success
            }
            Ok(Err(())) => ActionResult::ActionError(HandleRequestError::RequestNotFound),
        }
    }

    async fn resign(&self, game_id: GameId, player_id: PlayerId) -> Result<(), PlayerActionError> {
        let now = Instant::now();
        match self
            .handle_game_action_result(self.game_service.resign(game_id, player_id, now))
            .await?
        {
            ResignResult::GameOver(ended_game) => {
                self.handle_ended_game(ended_game).await;
                Ok(())
            }
        }
    }
}
