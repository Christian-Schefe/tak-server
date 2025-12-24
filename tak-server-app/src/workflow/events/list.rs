use std::sync::Arc;

use crate::domain::event::{Event, EventRepository};

#[async_trait::async_trait]
pub trait ListEventsUseCase {
    async fn list_events(&self) -> Vec<Event>;
}

pub struct ListEventsUseCaseImpl<R: EventRepository> {
    event_repository: Arc<R>,
}

impl<R: EventRepository> ListEventsUseCaseImpl<R> {
    pub fn new(event_repository: Arc<R>) -> Self {
        Self { event_repository }
    }
}

#[async_trait::async_trait]
impl<R: EventRepository + Send + Sync + 'static> ListEventsUseCase for ListEventsUseCaseImpl<R> {
    async fn list_events(&self) -> Vec<Event> {
        match self.event_repository.get_events().await {
            Ok(events) => events,
            Err(_) => {
                //TODO: log error
                Vec::new()
            }
        }
    }
}
