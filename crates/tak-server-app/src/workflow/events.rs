use std::sync::Arc;

use tak_domain::event::Event;

use crate::{PortsCollection, ports::event::EventRepository};

#[async_trait::async_trait]
pub trait EventsUseCase {
    async fn list_events(&self) -> Result<Vec<Event>, ListEventsError>;
}

pub enum ListEventsError {
    RepositoryError,
}

pub struct EventsUseCaseImpl<Ports: PortsCollection> {
    ports: Arc<Ports>,
}

impl<Ports: PortsCollection> EventsUseCaseImpl<Ports> {
    pub fn new(ports: Arc<Ports>) -> Self {
        Self { ports }
    }
}

#[async_trait::async_trait]
impl<Ports: PortsCollection + Send + Sync + 'static> EventsUseCase for EventsUseCaseImpl<Ports> {
    async fn list_events(&self) -> Result<Vec<Event>, ListEventsError> {
        match self.ports.event_repository().get_events().await {
            Ok(events) => Ok(events),
            Err(e) => {
                tracing::error!("Error retrieving events: {}", e);
                Err(ListEventsError::RepositoryError)
            }
        }
    }
}
