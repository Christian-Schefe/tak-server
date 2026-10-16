#[async_trait::async_trait]
pub trait EventRepository {
    async fn get_events(&self) -> Result<Vec<Event>, GetEventsError>;
}

pub enum GetEventsError {
    RetrievalError(String),
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
