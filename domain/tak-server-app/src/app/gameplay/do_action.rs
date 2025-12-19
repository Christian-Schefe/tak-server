use std::sync::Arc;

use tak_core::TakAction;

use crate::{
    app::{
        event::EventDispatcher,
        ports::{
            connection::PlayerConnectionPort,
            notification::{ListenerMessage, ListenerNotificationPort},
        },
    },
    domain::{
        GameId, PlayerId,
        game::{GameEvent, GameService},
    },
};

pub trait DoActionUseCase {
    fn do_action(
        &self,
        game_id: GameId,
        player_id: PlayerId,
        action: TakAction,
    ) -> Result<(), DoActionError>;
}

pub enum DoActionError {
    GameNotFound,
    NotPlayersGame,
    NotPlayersTurn,
    InvalidAction,
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
        let opponent_id = if game.white == player_id {
            game.black
        } else if game.black == player_id {
            game.white
        } else {
            return Err(DoActionError::NotPlayersGame);
        };

        let action_record = match self.game_service.do_action(game_id, player_id, action) {
            Ok(action_record) => action_record,
            Err(crate::domain::game::DoActionError::GameNotFound) => {
                return Err(DoActionError::GameNotFound);
            }
            Err(crate::domain::game::DoActionError::NotPlayersTurn) => {
                return Err(DoActionError::NotPlayersTurn);
            }
            Err(crate::domain::game::DoActionError::InvalidAction) => {
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
}
