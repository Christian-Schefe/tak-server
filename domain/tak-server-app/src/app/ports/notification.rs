use crate::{app::matchmaking::SeekView, domain::ListenerId};

pub trait ListenerNotificationPort {
    fn notify_listener(&self, listener: ListenerId, message: ListenerMessage);
    fn notify_listeners(&self, listeners: &[ListenerId], message: ListenerMessage);
    fn notify_all(&self, message: ListenerMessage);
}

pub enum ListenerMessage {
    SeekCreated { seek: SeekView },
    SeekCanceled { seek: SeekView },
}
