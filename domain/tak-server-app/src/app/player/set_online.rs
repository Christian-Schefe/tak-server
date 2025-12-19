use std::sync::Arc;

use crate::{
    app::event::EventDispatcher,
    domain::{
        PlayerId,
        player::{PlayerEvent, PlayerService},
        spectator::SpectatorService,
    },
};

pub trait SetPlayerOnlineUseCase {
    fn set_online(&self, player_id: PlayerId);
    fn set_offline(&self, player_id: PlayerId);
}

pub struct SetPlayerOnlineUseCaseImpl<
    P: PlayerService,
    PD: EventDispatcher<PlayerEvent>,
    S: SpectatorService,
> {
    player_service: Arc<P>,
    player_event_dispatcher: Arc<PD>,
    spectator_service: Arc<S>,
}

impl<P: PlayerService, PD: EventDispatcher<PlayerEvent>, S: SpectatorService>
    SetPlayerOnlineUseCaseImpl<P, PD, S>
{
    pub fn new(
        player_service: Arc<P>,
        player_event_dispatcher: Arc<PD>,
        spectator_service: Arc<S>,
    ) -> Self {
        Self {
            player_service,
            player_event_dispatcher,
            spectator_service,
        }
    }
}

impl<P: PlayerService, PD: EventDispatcher<PlayerEvent>, S: SpectatorService> SetPlayerOnlineUseCase
    for SetPlayerOnlineUseCaseImpl<P, PD, S>
{
    fn set_online(&self, player_id: PlayerId) {
        self.player_service.set_player_online(player_id);

        let events = self.player_service.take_events();
        self.player_event_dispatcher.handle_events(events);
    }

    fn set_offline(&self, player_id: PlayerId) {
        self.player_service.set_player_offline(player_id);
        self.spectator_service.unobserve_all_games(player_id);

        let events = self.player_service.take_events();
        self.player_event_dispatcher.handle_events(events);
    }
}
