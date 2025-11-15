use std::sync::Arc;

use crate::ServiceResult;

pub type ArcEventRepository = Arc<Box<dyn EventRepository + Send + Sync>>;

#[async_trait::async_trait]
pub trait EventRepository {
    async fn get_all_events(&self) -> ServiceResult<Vec<Event>>;
}

pub struct Event {
    pub name: String,
    pub event: String,
    pub category: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub details: Option<String>,
    pub registration: Option<String>,
    pub standings: Option<String>,
}

pub type ArcEventService = Arc<Box<dyn EventService + Send + Sync>>;

#[async_trait::async_trait]
pub trait EventService {
    async fn list_events(&self) -> ServiceResult<Vec<Event>>;
}

pub struct EventServiceImpl {
    repository: ArcEventRepository,
}

impl EventServiceImpl {
    pub fn new(repository: ArcEventRepository) -> Self {
        Self { repository }
    }
}

#[async_trait::async_trait]
impl EventService for EventServiceImpl {
    async fn list_events(&self) -> ServiceResult<Vec<Event>> {
        self.repository.get_all_events().await
    }
}
