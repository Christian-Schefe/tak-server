use std::sync::Arc;

use crate::{
    domain::{PlayerId, player::PlayerRepository},
    ports::authentication::{AuthenticationService, SessionToken, SubjectId},
};

pub trait LoginAccountUseCase {
    fn login_password(&self, username: String, password: String) -> Result<PlayerId, LoginError>;
    fn login_token(&self, token: &SessionToken) -> Result<PlayerId, LoginError>;
}

pub enum LoginError {
    InvalidCredentials,
}

pub struct LoginAccountUseCaseImpl<A: AuthenticationService, PR: PlayerRepository> {
    authentication_service: Arc<A>,
    player_repository: Arc<PR>,
}

impl<A: AuthenticationService, PR: PlayerRepository> LoginAccountUseCaseImpl<A, PR> {
    pub fn new(authentication_service: Arc<A>, player_repository: Arc<PR>) -> Self {
        Self {
            authentication_service,
            player_repository,
        }
    }
}

impl<A: AuthenticationService, PR: PlayerRepository> LoginAccountUseCase
    for LoginAccountUseCaseImpl<A, PR>
{
    fn login_password(&self, username: String, password: String) -> Result<PlayerId, LoginError> {
        let Ok(subject_id) = self
            .authentication_service
            .authenticate_username_password(&username, &password)
        else {
            return Err(LoginError::InvalidCredentials);
        };
        let SubjectId::Account(account_id) = subject_id;
        let Some(player) = self.player_repository.get_player_by_account_id(account_id) else {
            return Err(LoginError::InvalidCredentials);
        };
        Ok(player.player_id)
    }

    fn login_token(&self, token: &SessionToken) -> Result<PlayerId, LoginError> {
        let Ok(subject_id) = self
            .authentication_service
            .authenticate_session_token(&token)
        else {
            return Err(LoginError::InvalidCredentials);
        };
        let SubjectId::Account(account_id) = subject_id;
        let Some(player) = self.player_repository.get_player_by_account_id(account_id) else {
            return Err(LoginError::InvalidCredentials);
        };
        Ok(player.player_id)
    }
}
