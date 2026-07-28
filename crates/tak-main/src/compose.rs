use std::sync::Arc;

use tak_server_app::{
    domain::ListenerId,
    ports::notification::{ListenerMessage, ListenerNotificationPort},
};

pub struct ComposedListenerNotificationService {
    services: Vec<Arc<dyn ListenerNotificationPort + Send + Sync>>,
}

impl ComposedListenerNotificationService {
    pub fn new(services: Vec<Arc<dyn ListenerNotificationPort + Send + Sync>>) -> Self {
        Self { services }
    }
}

impl ListenerNotificationPort for ComposedListenerNotificationService {
    fn notify_listener(&self, listener: ListenerId, message: &ListenerMessage) {
        for service in &self.services {
            service.notify_listener(listener, message);
        }
    }
    fn notify_listeners(&self, listeners: &[ListenerId], message: &ListenerMessage) {
        for service in &self.services {
            service.notify_listeners(listeners, message);
        }
    }
    fn notify_all(&self, message: &ListenerMessage) {
        for service in &self.services {
            service.notify_all(message);
        }
    }
}
