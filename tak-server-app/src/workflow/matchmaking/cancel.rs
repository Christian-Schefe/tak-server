use std::sync::Arc;

use crate::{
    domain::{PlayerId, seek::SeekService},
    ports::notification::{ListenerMessage, ListenerNotificationPort},
};

pub trait CancelSeekUseCase {
    fn cancel_seek(&self, player: PlayerId);
}

pub struct CancelSeekUseCaseImpl<S: SeekService, L: ListenerNotificationPort> {
    seek_service: Arc<S>,
    notification_port: Arc<L>,
}

impl<S: SeekService, L: ListenerNotificationPort> CancelSeekUseCaseImpl<S, L> {
    pub fn new(seek_service: Arc<S>, notification_port: Arc<L>) -> Self {
        Self {
            seek_service,
            notification_port,
        }
    }
}

impl<S: SeekService, L: ListenerNotificationPort> CancelSeekUseCase
    for CancelSeekUseCaseImpl<S, L>
{
    fn cancel_seek(&self, player: PlayerId) {
        for cancelled_seek in self.seek_service.cancel_all_player_seeks(player) {
            let message = ListenerMessage::SeekCanceled {
                seek: cancelled_seek.into(),
            };
            self.notification_port.notify_all(message);
        }
    }
}
