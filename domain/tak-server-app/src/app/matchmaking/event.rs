use crate::domain::seek::SeekEvent;

pub trait SeekEventListener {
    fn on_seek_event(&self, seek_event: &SeekEvent);
}

pub trait SeekEventDispatcher {
    fn register_listener(&mut self, listener: Box<dyn SeekEventListener>);
    fn handle_events(&self, events: Vec<SeekEvent>);
}

pub struct InMemorySeekEventDispatcher {
    listeners: Vec<Box<dyn SeekEventListener>>,
}

impl InMemorySeekEventDispatcher {
    pub fn new() -> Self {
        Self {
            listeners: Vec::new(),
        }
    }
}

impl SeekEventDispatcher for InMemorySeekEventDispatcher {
    fn register_listener(&mut self, listener: Box<dyn SeekEventListener>) {
        self.listeners.push(listener);
    }

    fn handle_events(&self, events: Vec<SeekEvent>) {
        for event in events {
            for listener in &self.listeners {
                listener.on_seek_event(&event);
            }
        }
    }
}
