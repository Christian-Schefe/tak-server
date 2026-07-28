use std::sync::Arc;

use crate::domain::{ListenerId, chat::ChatRoomService, spectator::SpectatorService};

pub trait ListenerDisconnectUseCase {
    fn handle_listener_disconnect(&self, listener_id: ListenerId);
}

pub struct ListenerDisconnectUseCaseImpl<S: SpectatorService, C: ChatRoomService> {
    spectator_service: Arc<S>,
    chat_room_service: Arc<C>,
}

impl<S: SpectatorService, C: ChatRoomService> ListenerDisconnectUseCaseImpl<S, C> {
    pub fn new(spectator_service: Arc<S>, chat_room_service: Arc<C>) -> Self {
        Self {
            spectator_service,
            chat_room_service,
        }
    }
}

impl<S: SpectatorService, C: ChatRoomService> ListenerDisconnectUseCase
    for ListenerDisconnectUseCaseImpl<S, C>
{
    fn handle_listener_disconnect(&self, listener_id: ListenerId) {
        self.spectator_service.unobserve_all_games(listener_id);
        self.chat_room_service.leave_all_rooms(listener_id);
    }
}
