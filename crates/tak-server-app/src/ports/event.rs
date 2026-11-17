use tak_domain::event::Event;

use crate::errors::{AppResult, InternalError};

#[async_trait::async_trait]
pub trait EventRepository {
    async fn get_events(&self) -> AppResult<Vec<Event>, (InternalError,)>;
}
