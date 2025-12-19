pub trait EventListener<T> {
    fn on_event(&self, event: &T);
}

pub trait EventDispatcher<T> {
    fn register_listener(&mut self, listener: Box<dyn EventListener<T>>);
    fn handle_events(&self, events: Vec<T>);
}

pub struct InMemoryEventDispatcher<T> {
    listeners: Vec<Box<dyn EventListener<T>>>,
}

impl<T> InMemoryEventDispatcher<T> {
    pub fn new() -> Self {
        Self {
            listeners: Vec::new(),
        }
    }
}

impl<T> EventDispatcher<T> for InMemoryEventDispatcher<T> {
    fn register_listener(&mut self, listener: Box<dyn EventListener<T>>) {
        self.listeners.push(listener);
    }

    fn handle_events(&self, events: Vec<T>) {
        for event in events {
            for listener in &self.listeners {
                listener.on_event(&event);
            }
        }
    }
}
