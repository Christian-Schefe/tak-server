use std::sync::Arc;

use crate::{
    domain::player::{Player, PlayerRepository},
    ports::authentication::AuthContext,
};

pub trait RegisterAccountUseCase {
    fn create_player_if_not_exists(&self, auth_context: &AuthContext) -> bool;
}

pub struct RegisterAccountUseCaseImpl<PR: PlayerRepository> {
    player_repository: Arc<PR>,
}

impl<PR: PlayerRepository> RegisterAccountUseCaseImpl<PR> {
    pub fn new(player_repository: Arc<PR>) -> Self {
        Self { player_repository }
    }
}

impl<PR: PlayerRepository> RegisterAccountUseCase for RegisterAccountUseCaseImpl<PR> {
    fn create_player_if_not_exists(&self, auth_context: &AuthContext) -> bool {
        if self
            .player_repository
            .get_player_by_account_id(auth_context.account_id)
            .is_none()
        {
            let player = Player::new();
            if let Err(_) = self
                .player_repository
                .create_player(player, Some(auth_context.account_id))
            {
                //TODO: log error
                return false;
            }
            true
        } else {
            false
        }
    }
}
