use std::{sync::Arc, time::Instant};

use tak_core::TakAction;

use crate::{
    domain::{
        GameId, PlayerId,
        game::{
            DoActionResult, FinishedGame, GameService, OfferDrawResult, RequestUndoResult,
            ResignResult,
        },
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
    ) -> Result<(), DoActionError>;
    async fn offer_draw(
        &self,
        game_id: GameId,
        player_id: PlayerId,
        offer_status: bool,
    ) -> Result<(), OfferDrawError>;
    async fn request_undo(
        &self,
        game_id: GameId,
        player_id: PlayerId,
        offer_status: bool,
    ) -> Result<(), RequestUndoError>;
    async fn resign(&self, game_id: GameId, player_id: PlayerId) -> Result<(), ResignError>;
}

#[derive(Debug)]
pub enum DoActionError {
    GameNotFound,
    NotAPlayerInGame,
    InvalidAction(tak_core::InvalidActionReason),
    NotPlayersTurn,
    GameAlreadyEnded,
}

#[derive(Debug)]
pub enum OfferDrawError {
    GameNotFound,
    NotAPlayerInGame,
    GameAlreadyEnded,
}

#[derive(Debug)]
pub enum RequestUndoError {
    GameNotFound,
    NotAPlayerInGame,
    GameAlreadyEnded,
}

#[derive(Debug)]
pub enum ResignError {
    GameNotFound,
    NotAPlayerInGame,
    GameAlreadyEnded,
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
    ) -> Result<(), DoActionError> {
        log::debug!(
            "Player {} is performing action {:?} in game {}",
            player_id,
            action,
            game_id
        );
        let now = Instant::now();
        let (action_record, maybe_ended_game) =
            match self.game_service.do_action(game_id, player_id, action, now) {
                DoActionResult::ActionPerformed(action_record) => (action_record, None),
                DoActionResult::GameOver(action_record, ended_game) => {
                    (action_record, Some(ended_game))
                }
                DoActionResult::Timeout(ended_game) => {
                    self.handle_ended_game(ended_game).await;
                    return Err(DoActionError::GameAlreadyEnded);
                }
                DoActionResult::NotAPlayerInGame => {
                    return Err(DoActionError::NotAPlayerInGame);
                }
                DoActionResult::InvalidAction(e) => {
                    return Err(DoActionError::InvalidAction(e));
                }
                DoActionResult::NotPlayersTurn => {
                    return Err(DoActionError::NotPlayersTurn);
                }
                DoActionResult::GameNotFound => {
                    return Err(DoActionError::GameNotFound);
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

        Ok(())
    }

    async fn offer_draw(
        &self,
        game_id: GameId,
        player_id: PlayerId,
        offer_status: bool,
    ) -> Result<(), OfferDrawError> {
        let now = Instant::now();
        match self
            .game_service
            .offer_draw(game_id, player_id, now, offer_status)
        {
            OfferDrawResult::Success => {
                let msg = if offer_status {
                    ListenerMessage::GameDrawOffered {
                        game_id,
                        offering_player_id: player_id,
                    }
                } else {
                    ListenerMessage::GameDrawOfferRetracted {
                        game_id,
                        retracting_player_id: player_id,
                    }
                };
                self.notify_player_workflow
                    .notify_players_and_observers(game_id, &msg)
                    .await;
            }
            OfferDrawResult::NotAPlayerInGame => {
                return Err(OfferDrawError::NotAPlayerInGame);
            }
            OfferDrawResult::Unchanged => {}
            OfferDrawResult::GameDrawn(ended_game) => {
                self.handle_ended_game(ended_game).await;
            }
            OfferDrawResult::GameNotFound => return Err(OfferDrawError::GameNotFound),
            OfferDrawResult::Timeout(ended_game) => {
                self.handle_ended_game(ended_game).await;
                return Err(OfferDrawError::GameAlreadyEnded);
            }
        }

        Ok(())
    }

    async fn request_undo(
        &self,
        game_id: GameId,
        player_id: PlayerId,
        request_status: bool,
    ) -> Result<(), RequestUndoError> {
        let now = Instant::now();
        match self
            .game_service
            .request_undo(game_id, player_id, now, request_status)
        {
            RequestUndoResult::MoveUndone => {
                let msg = ListenerMessage::GameActionUndone { game_id };
                self.notify_player_workflow
                    .notify_players_and_observers(game_id, &msg)
                    .await;
                self.send_game_time_update(game_id, now).await;
            }
            RequestUndoResult::Success => {
                let msg = if request_status {
                    ListenerMessage::GameUndoRequested {
                        game_id,
                        requesting_player_id: player_id,
                    }
                } else {
                    ListenerMessage::GameUndoRequestRetracted {
                        game_id,
                        retracting_player_id: player_id,
                    }
                };
                self.notify_player_workflow
                    .notify_players_and_observers(game_id, &msg)
                    .await;
            }
            RequestUndoResult::Unchanged => {}
            RequestUndoResult::NotAPlayerInGame => {
                return Err(RequestUndoError::NotAPlayerInGame);
            }
            RequestUndoResult::GameNotFound => return Err(RequestUndoError::GameNotFound),
            RequestUndoResult::Timeout(ended_game) => {
                self.handle_ended_game(ended_game).await;
                return Err(RequestUndoError::GameAlreadyEnded);
            }
        }

        Ok(())
    }

    async fn resign(&self, game_id: GameId, player_id: PlayerId) -> Result<(), ResignError> {
        let now = Instant::now();
        match self.game_service.resign(game_id, player_id, now) {
            ResignResult::GameOver(ended_game) => {
                self.handle_ended_game(ended_game).await;
            }
            ResignResult::Timeout(ended_game) => {
                self.handle_ended_game(ended_game).await;
                return Err(ResignError::GameAlreadyEnded);
            }
            ResignResult::NotAPlayerInGame => {
                return Err(ResignError::NotAPlayerInGame);
            }
            ResignResult::GameNotFound => return Err(ResignError::GameNotFound),
        }

        Ok(())
    }
}
