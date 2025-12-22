use std::sync::Arc;

use crate::{
    app::event::EventDispatcher,
    domain::{
        PlayerId,
        chat::ChatRoomService,
        player::{PlayerEvent, PlayerService},
        spectator::SpectatorService,
    },
    ports::connection::PlayerConnectionPort,
};

pub trait SetPlayerOnlineUseCase {
    fn set_online(&self, player_id: PlayerId);
    fn set_offline(&self, player_id: PlayerId);
}

pub struct SetPlayerOnlineUseCaseImpl<
    P: PlayerService,
    PD: EventDispatcher<PlayerEvent>,
    S: SpectatorService,
    PC: PlayerConnectionPort,
    C: ChatRoomService,
> {
    player_service: Arc<P>,
    player_event_dispatcher: Arc<PD>,
    spectator_service: Arc<S>,
    player_connection_port: Arc<PC>,
    chat_room_service: Arc<C>,
}

impl<
    P: PlayerService,
    PD: EventDispatcher<PlayerEvent>,
    S: SpectatorService,
    PC: PlayerConnectionPort,
    C: ChatRoomService,
> SetPlayerOnlineUseCaseImpl<P, PD, S, PC, C>
{
    pub fn new(
        player_service: Arc<P>,
        player_event_dispatcher: Arc<PD>,
        spectator_service: Arc<S>,
        player_connection_port: Arc<PC>,
        chat_room_service: Arc<C>,
    ) -> Self {
        Self {
            player_service,
            player_event_dispatcher,
            spectator_service,
            player_connection_port,
            chat_room_service,
        }
    }
}

impl<
    P: PlayerService,
    PD: EventDispatcher<PlayerEvent>,
    S: SpectatorService,
    PC: PlayerConnectionPort,
    C: ChatRoomService,
> SetPlayerOnlineUseCase for SetPlayerOnlineUseCaseImpl<P, PD, S, PC, C>
{
    fn set_online(&self, player_id: PlayerId) {
        self.player_service.set_player_online(player_id);

        let events = self.player_service.take_events();
        self.player_event_dispatcher.handle_events(events);
    }

    fn set_offline(&self, player_id: PlayerId) {
        self.player_service.set_player_offline(player_id);
        if let Some(listener_id) = self.player_connection_port.get_connection_id(player_id) {
            self.spectator_service.unobserve_all_games(listener_id);
            self.chat_room_service.leave_all_rooms(listener_id);
        }

        let events = self.player_service.take_events();
        self.player_event_dispatcher.handle_events(events);
    }
}
