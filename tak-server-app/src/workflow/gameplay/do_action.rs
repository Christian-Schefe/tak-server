use std::sync::Arc;

use tak_core::TakAction;

use crate::{
    domain::{
        GameId, PlayerId,
        game::{
            DoActionError, DoActionSuccess, GameService, OfferDrawError, OfferDrawSuccess,
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
        let (action_record, maybe_ended_game) =
            match self.game_service.do_action(game_id, player_id, action)? {
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
        self.notify_player_workflow
            .notify_players_and_observers(game_id, msg)
            .await;

        if let Some(ended_game) = maybe_ended_game {
            self.finalize_game_workflow.finalize_game(ended_game).await;
        }

        Ok(())
    }

    async fn offer_draw(&self, game_id: GameId, player_id: PlayerId) -> Result<(), OfferDrawError> {
        match self.game_service.offer_draw(game_id, player_id)? {
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
        let did_retract = self.game_service.retract_draw_offer(game_id, player_id)?;

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
        match self.game_service.request_undo(game_id, player_id)? {
            RequestUndoSuccess::MoveUndone => {
                let msg = ListenerMessage::GameActionUndone { game_id };
                self.notify_player_workflow
                    .notify_players_and_observers(game_id, msg)
                    .await;
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
        let did_retract = self.game_service.retract_undo_request(game_id, player_id)?;

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
        let ended_game = self.game_service.resign(game_id, player_id)?;

        self.finalize_game_workflow.finalize_game(ended_game).await;

        Ok(())
    }
}
