use std::sync::Arc;

use crate::{
    domain::player::{CreatePlayerError, Player, PlayerRepository},
    ports::authentication::AuthContext,
};

#[async_trait::async_trait]
pub trait RegisterAccountUseCase {
    async fn create_player_if_not_exists(&self, auth_context: &AuthContext) -> bool;
}

pub struct RegisterAccountUseCaseImpl<PR: PlayerRepository> {
    player_repository: Arc<PR>,
}

impl<PR: PlayerRepository> RegisterAccountUseCaseImpl<PR> {
    pub fn new(player_repository: Arc<PR>) -> Self {
        Self { player_repository }
    }
}

#[async_trait::async_trait]
impl<PR: PlayerRepository + Send + Sync + 'static> RegisterAccountUseCase
    for RegisterAccountUseCaseImpl<PR>
{
    async fn create_player_if_not_exists(&self, auth_context: &AuthContext) -> bool {
        let player = Player::new();
        match self
            .player_repository
            .create_player(player, Some(auth_context.account_id))
            .await
        {
            Ok(()) => true,
            Err(CreatePlayerError::PlayerAlreadyExists) => false,
            Err(CreatePlayerError::StorageError(e)) => {
                log::error!("Failed to create player: {}", e);
                false
            }
        }
    }
}
