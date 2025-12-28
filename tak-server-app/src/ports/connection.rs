use crate::domain::{ListenerId, PlayerId};

#[async_trait::async_trait]
pub trait PlayerConnectionPort {
    async fn get_connection_id(&self, player_id: PlayerId) -> Option<ListenerId>;
}
