use std::sync::Arc;

use tak_core::TakAction;

use crate::app::{
    domain::{
        GameId, PlayerId,
        game::{GameEvent, GameService},
    },
    ports::{
        connection::PlayerConnectionPort,
        notification::{ListenerMessage, ListenerNotificationPort},
    },
    workflow::event::EventDispatcher,
};

pub trait DoActionUseCase {
    fn do_action(
        &self,
        game_id: GameId,
        player_id: PlayerId,
        action: TakAction,
    ) -> Result<(), DoActionError>;
    fn offer_draw(&self, game_id: GameId, player_id: PlayerId) -> Result<(), OfferDrawError>;
    fn request_undo(&self, game_id: GameId, player_id: PlayerId) -> Result<(), RequestUndoError>;
    fn resign(&self, game_id: GameId, player_id: PlayerId) -> Result<(), ResignError>;
}

pub enum DoActionError {
    GameNotFound,
    NotPlayersTurn,
    InvalidAction,
}

pub enum OfferDrawError {
    GameNotFound,
}

pub enum RequestUndoError {
    GameNotFound,
    CantUndo,
}

pub enum ResignError {
    GameNotFound,
}

pub struct DoActionUseCaseImpl<
    G: GameService,
    L: ListenerNotificationPort,
    C: PlayerConnectionPort,
    GD: EventDispatcher<GameEvent>,
> {
    game_service: Arc<G>,
    listener_notification_port: Arc<L>,
    player_connection_port: Arc<C>,
    game_event_dispatcher: Arc<GD>,
}

impl<
    G: GameService,
    L: ListenerNotificationPort,
    C: PlayerConnectionPort,
    GD: EventDispatcher<GameEvent>,
> DoActionUseCaseImpl<G, L, C, GD>
{
    pub fn new(
        game_service: Arc<G>,
        listener_notification_port: Arc<L>,
        player_connection_port: Arc<C>,
        game_event_dispatcher: Arc<GD>,
    ) -> Self {
        Self {
            game_service,
            listener_notification_port,
            player_connection_port,
            game_event_dispatcher,
        }
    }
}

impl<
    G: GameService,
    L: ListenerNotificationPort,
    C: PlayerConnectionPort,
    GD: EventDispatcher<GameEvent>,
> DoActionUseCase for DoActionUseCaseImpl<G, L, C, GD>
{
    fn do_action(
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

        let action_record = match self.game_service.do_action(game_id, player_id, action) {
            Ok(action_record) => action_record,
            Err(crate::app::domain::game::DoActionError::GameNotFound) => {
                return Err(DoActionError::GameNotFound);
            }
            Err(crate::app::domain::game::DoActionError::NotPlayersTurn) => {
                return Err(DoActionError::NotPlayersTurn);
            }
            Err(crate::app::domain::game::DoActionError::InvalidAction) => {
                return Err(DoActionError::InvalidAction);
            }
        };

        if let Some(opponent_connection) =
            self.player_connection_port.get_connection_id(opponent_id)
        {
            let msg = ListenerMessage::GameAction {
                game_id,
                action: action_record,
            };
            self.listener_notification_port
                .notify_listener(opponent_connection, msg);
        }

        let events = self.game_service.take_events();
        self.game_event_dispatcher.handle_events(events);

        Ok(())
    }

    fn offer_draw(&self, game_id: GameId, player_id: PlayerId) -> Result<(), OfferDrawError> {
        let Some(game) = self.game_service.get_game_by_id(game_id) else {
            return Err(OfferDrawError::GameNotFound);
        };

        let Some(opponent_id) = game.get_opponent(player_id) else {
            return Err(OfferDrawError::GameNotFound);
        };

        let did_draw = self
            .game_service
            .offer_draw(game_id, player_id)
            .map_err(|_| OfferDrawError::GameNotFound)?;

        if !did_draw {
            if let Some(opponent_connection) =
                self.player_connection_port.get_connection_id(opponent_id)
            {
                let msg = ListenerMessage::GameDrawOffered { game_id };
                self.listener_notification_port
                    .notify_listener(opponent_connection, msg);
            }
        }
        Ok(())
    }

    fn request_undo(&self, game_id: GameId, player_id: PlayerId) -> Result<(), RequestUndoError> {
        let Some(game) = self.game_service.get_game_by_id(game_id) else {
            return Err(RequestUndoError::GameNotFound);
        };

        let Some(opponent_id) = game.get_opponent(player_id) else {
            return Err(RequestUndoError::GameNotFound);
        };

        let did_undo = self
            .game_service
            .request_undo(game_id, player_id)
            .map_err(|_| RequestUndoError::GameNotFound)?;

        if !did_undo {
            if let Some(opponent_connection) =
                self.player_connection_port.get_connection_id(opponent_id)
            {
                let msg = ListenerMessage::GameUndoRequested { game_id };
                self.listener_notification_port
                    .notify_listener(opponent_connection, msg);
            }
        }
        Ok(())
    }

    fn resign(&self, game_id: GameId, player_id: PlayerId) -> Result<(), ResignError> {
        self.game_service
            .resign(game_id, player_id)
            .map_err(|_| ResignError::GameNotFound)?;

        let events = self.game_service.take_events();
        self.game_event_dispatcher.handle_events(events);

        Ok(())
    }
}
