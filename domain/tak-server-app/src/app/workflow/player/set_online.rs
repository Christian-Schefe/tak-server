use std::sync::Arc;

use crate::app::{
    domain::{
        PlayerId,
        player::{PlayerEvent, PlayerService},
    },
    workflow::event::EventDispatcher,
};

pub trait SetPlayerOnlineUseCase {
    fn set_online(&self, player_id: PlayerId);
    fn set_offline(&self, player_id: PlayerId);
}

pub struct SetPlayerOnlineUseCaseImpl<P: PlayerService, PD: EventDispatcher<PlayerEvent>> {
    player_service: Arc<P>,
    player_event_dispatcher: Arc<PD>,
}

impl<P: PlayerService, PD: EventDispatcher<PlayerEvent>> SetPlayerOnlineUseCaseImpl<P, PD> {
    pub fn new(player_service: Arc<P>, player_event_dispatcher: Arc<PD>) -> Self {
        Self {
            player_service,
            player_event_dispatcher,
        }
    }
}

impl<P: PlayerService, PD: EventDispatcher<PlayerEvent>> SetPlayerOnlineUseCase
    for SetPlayerOnlineUseCaseImpl<P, PD>
{
    fn set_online(&self, player_id: PlayerId) {
        self.player_service.set_player_online(player_id);

        let events = self.player_service.take_events();
        self.player_event_dispatcher.handle_events(events);
    }

    fn set_offline(&self, player_id: PlayerId) {
        self.player_service.set_player_offline(player_id);

        let events = self.player_service.take_events();
        self.player_event_dispatcher.handle_events(events);
    }
}
