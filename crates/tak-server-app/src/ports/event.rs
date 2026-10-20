use tak_domain::event::Event;

#[async_trait::async_trait]
pub trait EventRepository {
    async fn get_events(&self) -> Result<Vec<Event>, String>;
}
