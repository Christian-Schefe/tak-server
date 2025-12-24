use std::sync::Arc;

use crate::app::{
    domain::{
        PlayerId,
        seek::{SeekEvent, SeekService},
    },
    workflow::event::EventDispatcher,
};

pub trait CancelSeekUseCase {
    fn cancel_seek(&self, player: PlayerId);
}

pub struct CancelSeekUseCaseImpl<S: SeekService, SD: EventDispatcher<SeekEvent>> {
    seek_service: Arc<S>,
    seek_event_dispatcher: Arc<SD>,
}

impl<S: SeekService, SD: EventDispatcher<SeekEvent>> CancelSeekUseCaseImpl<S, SD> {
    pub fn new(seek_service: Arc<S>, seek_event_dispatcher: Arc<SD>) -> Self {
        Self {
            seek_service,
            seek_event_dispatcher,
        }
    }
}

impl<S: SeekService, SD: EventDispatcher<SeekEvent>> CancelSeekUseCase
    for CancelSeekUseCaseImpl<S, SD>
{
    fn cancel_seek(&self, player: PlayerId) {
        if let Some(seek_id) = self.seek_service.get_seek_by_player(player) {
            self.seek_service.cancel_seek(seek_id);
        }

        let events = self.seek_service.take_events();
        self.seek_event_dispatcher.handle_events(events);
    }
}
