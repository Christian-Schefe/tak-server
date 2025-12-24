use std::sync::Arc;

use crate::app::domain::event::{Event, EventRepository};

pub trait ListEventsUseCase {
    fn list_events(&self) -> Vec<Event>;
}

pub struct ListEventsUseCaseImpl<R: EventRepository> {
    event_repository: Arc<R>,
}

impl<R: EventRepository> ListEventsUseCaseImpl<R> {
    pub fn new(event_repository: Arc<R>) -> Self {
        Self { event_repository }
    }
}

impl<R: EventRepository> ListEventsUseCase for ListEventsUseCaseImpl<R> {
    fn list_events(&self) -> Vec<Event> {
        self.event_repository.get_events()
    }
}
