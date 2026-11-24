use std::sync::Arc;

use tak_domain::event::Event;

use crate::{
    PortsCollection,
    errors::{AppResult, InternalError},
};

#[async_trait::async_trait]
pub trait EventsUseCase {
    async fn list_events(&self) -> AppResult<Vec<Event>, (InternalError,)>;
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
    async fn list_events(&self) -> AppResult<Vec<Event>, (InternalError,)> {
        self.ports.event_repository().get_events().await
    }
}
