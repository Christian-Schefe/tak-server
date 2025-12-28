use std::sync::Arc;

use tak_core::TakAction;

use crate::{
    domain::{
        GameId, PlayerId,
        game::{
            DoActionError, DoActionSuccess, GameService, OfferDrawError, OfferDrawSuccess,
            RequestUndoError, ResignError,
        },
    },
    ports::{
        connection::PlayerConnectionPort,
        notification::{ListenerMessage, ListenerNotificationPort},
    },
    workflow::gameplay::finalize_game::FinalizeGameWorkflow,
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

pub struct DoActionUseCaseImpl<
    G: GameService,
    L: ListenerNotificationPort,
    C: PlayerConnectionPort,
    F: FinalizeGameWorkflow,
> {
    game_service: Arc<G>,
    listener_notification_port: Arc<L>,
    player_connection_port: Arc<C>,

    finalize_game_workflow: Arc<F>,
}

impl<G: GameService, L: ListenerNotificationPort, C: PlayerConnectionPort, F: FinalizeGameWorkflow>
    DoActionUseCaseImpl<G, L, C, F>
{
    pub fn new(
        game_service: Arc<G>,
        listener_notification_port: Arc<L>,
        player_connection_port: Arc<C>,
        finalize_game_workflow: Arc<F>,
    ) -> Self {
        Self {
            game_service,
            listener_notification_port,
            player_connection_port,
            finalize_game_workflow,
        }
    }
}

#[async_trait::async_trait]
impl<
    G: GameService + Send + Sync + 'static,
    L: ListenerNotificationPort + Send + Sync + 'static,
    C: PlayerConnectionPort + Send + Sync + 'static,
    F: FinalizeGameWorkflow + Send + Sync + 'static,
> DoActionUseCase for DoActionUseCaseImpl<G, L, C, F>
{
    async fn do_action(
        &self,
        game_id: GameId,
        player_id: PlayerId,
        action: TakAction,
    ) -> Result<(), DoActionError> {
        let Some(game) = self.game_service.get_game_by_id(game_id) else {
            return Err(DoActionError::GameNotFound);
        };
        let Some(opponent_id) = game.get_opponent(player_id) else {
            return Err(DoActionError::GameNotFound);
        };

        let (action_record, maybe_ended_game) =
            match self.game_service.do_action(game_id, player_id, action)? {
                DoActionSuccess::ActionPerformed(action_record) => (action_record, None),
                DoActionSuccess::GameOver(action_record, ended_game) => {
                    (action_record, Some(ended_game))
                }
            };

        if let Some(opponent_connection) = self
            .player_connection_port
            .get_connection_id(opponent_id)
            .await
        {
            let msg = ListenerMessage::GameAction {
                game_id,
                action: action_record,
            };
            self.listener_notification_port
                .notify_listener(opponent_connection, msg);
        }

        if let Some(ended_game) = maybe_ended_game {
            self.finalize_game_workflow.finalize_game(ended_game).await;
        }

        Ok(())
    }

    async fn offer_draw(&self, game_id: GameId, player_id: PlayerId) -> Result<(), OfferDrawError> {
        let Some(game) = self.game_service.get_game_by_id(game_id) else {
            return Err(OfferDrawError::GameNotFound);
        };

        let Some(opponent_id) = game.get_opponent(player_id) else {
            return Err(OfferDrawError::GameNotFound);
        };

        match self.game_service.offer_draw(game_id, player_id)? {
            OfferDrawSuccess::DrawOffered(changed) => {
                if changed
                    && let Some(opponent_connection) = self
                        .player_connection_port
                        .get_connection_id(opponent_id)
                        .await
                {
                    let msg = ListenerMessage::GameDrawOffered { game_id };
                    self.listener_notification_port
                        .notify_listener(opponent_connection, msg);
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
        let Some(game) = self.game_service.get_game_by_id(game_id) else {
            return Err(OfferDrawError::GameNotFound);
        };

        let Some(opponent_id) = game.get_opponent(player_id) else {
            return Err(OfferDrawError::GameNotFound);
        };

        let did_retract = self.game_service.retract_draw_offer(game_id, player_id)?;

        if did_retract {
            if let Some(opponent_connection) = self
                .player_connection_port
                .get_connection_id(opponent_id)
                .await
            {
                let msg = ListenerMessage::GameDrawOfferRetracted { game_id };
                self.listener_notification_port
                    .notify_listener(opponent_connection, msg);
            }
        }
        Ok(())
    }

    async fn request_undo(
        &self,
        game_id: GameId,
        player_id: PlayerId,
    ) -> Result<(), RequestUndoError> {
        let Some(game) = self.game_service.get_game_by_id(game_id) else {
            return Err(RequestUndoError::GameNotFound);
        };

        let Some(opponent_id) = game.get_opponent(player_id) else {
            return Err(RequestUndoError::GameNotFound);
        };

        let did_undo = self.game_service.request_undo(game_id, player_id)?;

        if !did_undo {
            if let Some(opponent_connection) = self
                .player_connection_port
                .get_connection_id(opponent_id)
                .await
            {
                let msg = ListenerMessage::GameUndoRequested { game_id };
                self.listener_notification_port
                    .notify_listener(opponent_connection, msg);
            }
        }
        Ok(())
    }

    async fn retract_undo_request(
        &self,
        game_id: GameId,
        player_id: PlayerId,
    ) -> Result<(), RequestUndoError> {
        let Some(game) = self.game_service.get_game_by_id(game_id) else {
            return Err(RequestUndoError::GameNotFound);
        };

        let Some(opponent_id) = game.get_opponent(player_id) else {
            return Err(RequestUndoError::GameNotFound);
        };

        let did_retract = self.game_service.retract_undo_request(game_id, player_id)?;

        if did_retract {
            if let Some(opponent_connection) = self
                .player_connection_port
                .get_connection_id(opponent_id)
                .await
            {
                let msg = ListenerMessage::GameUndoRequestRetracted { game_id };
                self.listener_notification_port
                    .notify_listener(opponent_connection, msg);
            }
        }
        Ok(())
    }

    async fn resign(&self, game_id: GameId, player_id: PlayerId) -> Result<(), ResignError> {
        let ended_game = self.game_service.resign(game_id, player_id)?;

        self.finalize_game_workflow.finalize_game(ended_game).await;

        Ok(())
    }
}
