use std::sync::Arc;

use tak_domain::event::Event;

use crate::ports::{RepoError, event::EventRepository};

#[async_trait::async_trait]
pub trait ListEventsUseCase {
    async fn list_events(&self) -> Result<Vec<Event>, ListEventsError>;
}

pub enum ListEventsError {
    RepositoryError,
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
    async fn list_events(&self) -> Result<Vec<Event>, ListEventsError> {
        match self.event_repository.get_events().await {
            Ok(events) => Ok(events),
            Err(RepoError::StorageError(e)) => {
                tracing::error!("Error retrieving events: {}", e);
                Err(ListEventsError::RepositoryError)
            }
        }
    }
}
