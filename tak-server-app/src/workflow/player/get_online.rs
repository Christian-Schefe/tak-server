use std::sync::Arc;

use crate::domain::{PlayerId, player::PlayerService};

pub trait GetOnlinePlayersUseCase {
    fn get_online_players(&self) -> Vec<PlayerId>;
}

pub struct GetOnlinePlayersUseCaseImpl<P: PlayerService> {
    player_service: Arc<P>,
}

impl<P> GetOnlinePlayersUseCaseImpl<P>
where
    P: PlayerService + Send + Sync + 'static,
{
    pub fn new(player_service: Arc<P>) -> Self {
        Self { player_service }
    }
}

impl<P> GetOnlinePlayersUseCase for GetOnlinePlayersUseCaseImpl<P>
where
    P: PlayerService + Send + Sync + 'static,
{
    fn get_online_players(&self) -> Vec<PlayerId> {
        self.player_service.get_online_players()
    }
}
