use std::sync::Arc;

use crate::{
    app::{
        matchmaking::event::SeekEventListener,
        ports::notification::{ListenerMessage, ListenerNotificationPort},
    },
    domain::seek::SeekEvent,
};

pub struct SeekEventNotifier<L: ListenerNotificationPort> {
    notification_port: Arc<L>,
}

impl<L: ListenerNotificationPort> SeekEventNotifier<L> {
    pub fn new(notification_port: Arc<L>) -> Self {
        Self { notification_port }
    }
}

impl<L: ListenerNotificationPort> SeekEventListener for SeekEventNotifier<L> {
    fn on_seek_event(&self, seek_event: &SeekEvent) {
        let message = match seek_event {
            SeekEvent::Created(seek) => ListenerMessage::SeekCreated { seek: seek.into() },
            SeekEvent::Canceled(seek) => ListenerMessage::SeekCanceled { seek: seek.into() },
        };
        self.notification_port.notify_all(message);
    }
}
