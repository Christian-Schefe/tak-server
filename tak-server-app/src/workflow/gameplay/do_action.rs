use std::{sync::Arc, time::Instant};

use tak_core::TakAction;

use crate::{
    domain::{
        GameId, PlayerId,
        game::{
            DoActionError, DoActionSuccess, Game, GameService, OfferDrawError, OfferDrawSuccess,
            RequestUndoError, RequestUndoSuccess, ResignError,
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
    async fn offer_draw(&self, game_id: GameId, player_id: PlayerId) -> Result<(), OfferDrawError>;
    async fn retract_draw_offer(
        &self,
        game_id: GameId,
        player_id: PlayerId,
    ) -> Result<(), OfferDrawError>;
    async fn request_undo(
        &self,
        game_id: GameId,
        player_id: PlayerId,
    ) -> Result<(), RequestUndoError>;
    async fn retract_undo_request(
        &self,
        game_id: GameId,
        player_id: PlayerId,
    ) -> Result<(), RequestUndoError>;
    async fn resign(&self, game_id: GameId, player_id: PlayerId) -> Result<(), ResignError>;
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
            self.send_game_time_update_for_game(&game, now).await;
        }
    }

    async fn send_game_time_update_for_game(&self, game: &Game, now: Instant) {
        let time_remaining = game.get_time_remaining(now);
        let time_update_msg = ListenerMessage::GameTimeUpdate {
            game_id: game.game_id,
            white_time: time_remaining.white_time,
            black_time: time_remaining.black_time,
        };
        self.notify_player_workflow
            .notify_players_and_observers_of_game(game, time_update_msg)
            .await;
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
        let (action_record, maybe_ended_game) = match self
            .game_service
            .do_action(game_id, player_id, action, now)?
        {
            DoActionSuccess::ActionPerformed(action_record) => (action_record, None),
            DoActionSuccess::GameOver(action_record, ended_game) => {
                (action_record, Some(ended_game))
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
                .notify_players_and_observers_of_game(&ended_game, msg)
                .await;

            self.send_game_time_update_for_game(&ended_game, now).await;

            self.finalize_game_workflow.finalize_game(ended_game).await;
        } else {
            self.notify_player_workflow
                .notify_players_and_observers(game_id, msg)
                .await;

            self.send_game_time_update(game_id, now).await;
        }

        Ok(())
    }

    async fn offer_draw(&self, game_id: GameId, player_id: PlayerId) -> Result<(), OfferDrawError> {
        let now = Instant::now();
        match self.game_service.offer_draw(game_id, player_id, now)? {
            OfferDrawSuccess::DrawOffered(changed) => {
                if changed {
                    let msg = ListenerMessage::GameDrawOffered {
                        game_id,
                        offering_player_id: player_id,
                    };
                    self.notify_player_workflow
                        .notify_players_and_observers(game_id, msg)
                        .await;
                }
            }
            OfferDrawSuccess::GameDrawn(ended_game) => {
                self.send_game_time_update_for_game(&ended_game, now).await;
                self.finalize_game_workflow.finalize_game(ended_game).await;
            }
        }

        Ok(())
    }

    async fn retract_draw_offer(
        &self,
        game_id: GameId,
        player_id: PlayerId,
    ) -> Result<(), OfferDrawError> {
        let now = Instant::now();
        let did_retract = self
            .game_service
            .retract_draw_offer(game_id, player_id, now)?;

        if did_retract {
            let msg = ListenerMessage::GameDrawOfferRetracted {
                game_id,
                retracting_player_id: player_id,
            };
            self.notify_player_workflow
                .notify_players_and_observers(game_id, msg)
                .await;
        }

        Ok(())
    }

    async fn request_undo(
        &self,
        game_id: GameId,
        player_id: PlayerId,
    ) -> Result<(), RequestUndoError> {
        let now = Instant::now();
        match self.game_service.request_undo(game_id, player_id, now)? {
            RequestUndoSuccess::MoveUndone => {
                let msg = ListenerMessage::GameActionUndone { game_id };
                self.notify_player_workflow
                    .notify_players_and_observers(game_id, msg)
                    .await;
                self.send_game_time_update(game_id, now).await;
            }
            RequestUndoSuccess::UndoRequested(changed) => {
                if changed {
                    let msg = ListenerMessage::GameUndoRequested {
                        game_id,
                        requesting_player_id: player_id,
                    };
                    self.notify_player_workflow
                        .notify_players_and_observers(game_id, msg)
                        .await;
                }
            }
        }

        Ok(())
    }

    async fn retract_undo_request(
        &self,
        game_id: GameId,
        player_id: PlayerId,
    ) -> Result<(), RequestUndoError> {
        let now = Instant::now();
        let did_retract = self
            .game_service
            .retract_undo_request(game_id, player_id, now)?;

        if did_retract {
            let msg = ListenerMessage::GameUndoRequestRetracted {
                game_id,
                retracting_player_id: player_id,
            };
            self.notify_player_workflow
                .notify_players_and_observers(game_id, msg)
                .await;
        }
        Ok(())
    }

    async fn resign(&self, game_id: GameId, player_id: PlayerId) -> Result<(), ResignError> {
        let now = Instant::now();
        let ended_game = self.game_service.resign(game_id, player_id, now)?;

        self.send_game_time_update_for_game(&ended_game, now).await;
        self.finalize_game_workflow.finalize_game(ended_game).await;

        Ok(())
    }
}
