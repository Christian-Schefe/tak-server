use std::sync::Arc;

use crate::{
    domain::{PlayerId, account::PermissionPolicy, player::PlayerRepository},
    ports::{
        authentication::{AuthSubject, AuthenticationService},
        email::EmailPort,
    },
};

pub trait ModeratePlayerUseCase {
    fn ban_player(
        &self,
        player_id: PlayerId,
        target_player_id: PlayerId,
        reason: &str,
    ) -> Result<(), ModerationError>;
    fn silence_player(
        &self,
        player_id: PlayerId,
        target_player_id: PlayerId,
    ) -> Result<(), ModerationError>;
}

pub enum ModerationError {
    PlayerNotFound,
    AccountNotFound,
    InsufficientPermissions,
}

pub struct ModeratePlayerUseCaseImpl<
    E: EmailPort,
    BPP: PermissionPolicy,
    SPP: PermissionPolicy,
    PR: PlayerRepository,
    A: AuthenticationService,
> {
    email_port: Arc<E>,
    ban_policy: Arc<BPP>,
    silence_policy: Arc<SPP>,
    player_repository: Arc<PR>,
    authentication_service: Arc<A>,
}

impl<
    E: EmailPort,
    BPP: PermissionPolicy,
    SPP: PermissionPolicy,
    PR: PlayerRepository,
    A: AuthenticationService,
> ModeratePlayerUseCaseImpl<E, BPP, SPP, PR, A>
{
    pub fn new(
        email_port: Arc<E>,
        ban_policy: Arc<BPP>,
        silence_policy: Arc<SPP>,
        player_repository: Arc<PR>,
        authentication_service: Arc<A>,
    ) -> Self {
        Self {
            email_port,
            ban_policy,
            silence_policy,
            player_repository,
            authentication_service,
        }
    }
}

impl<
    E: EmailPort,
    BPP: PermissionPolicy,
    SPP: PermissionPolicy,
    PR: PlayerRepository,
    A: AuthenticationService,
> ModeratePlayerUseCase for ModeratePlayerUseCaseImpl<E, BPP, SPP, PR, A>
{
    fn ban_player(
        &self,
        player_id: PlayerId,
        target_player_id: PlayerId,
        reason: &str,
    ) -> Result<(), ModerationError> {
        let Some(executing_account_id) = self.player_repository.get_account_for_player(player_id)
        else {
            return Err(ModerationError::PlayerNotFound);
        };
        let Some(target_account_id) = self
            .player_repository
            .get_account_for_player(target_player_id)
        else {
            return Err(ModerationError::PlayerNotFound);
        };

        let Some(executing_account) = self
            .authentication_service
            .get_subject(executing_account_id)
        else {
            return Err(ModerationError::AccountNotFound);
        };
        let Some(target_account) = self.authentication_service.get_subject(target_account_id)
        else {
            return Err(ModerationError::AccountNotFound);
        };

        if !self
            .ban_policy
            .has_permissions(&executing_account.role, &target_account.role)
        {
            return Err(ModerationError::InsufficientPermissions);
        }

        self.player_repository.set_player_banned(player_id, true);

        if let AuthSubject::Player { username, email } = target_account.subject_type {
            let subject = "Playtak Account Banned";
            let body = format!(
                "Hello {},\n\n\
        Your account has been banned for the following reason:\n\
        {}\n\n\
        If you believe this is a mistake, please contact support.\n\n\
        Best regards,\n\
        The Playtak Team",
                username, reason
            );
            if let Some(email) = email {
                match self.email_port.send_email(&email, &subject, &body) {
                    Ok(_) => {}
                    Err(_) => {} // Ok if email fails, we still ban the account
                }
            }
        }

        Ok(())
    }

    fn silence_player(
        &self,
        player_id: PlayerId,
        target_player_id: PlayerId,
    ) -> Result<(), ModerationError> {
        let Some(executing_account_id) = self.player_repository.get_account_for_player(player_id)
        else {
            return Err(ModerationError::PlayerNotFound);
        };
        let Some(target_account_id) = self
            .player_repository
            .get_account_for_player(target_player_id)
        else {
            return Err(ModerationError::PlayerNotFound);
        };

        let Some(executing_account) = self
            .authentication_service
            .get_subject(executing_account_id)
        else {
            return Err(ModerationError::AccountNotFound);
        };
        let Some(target_account) = self.authentication_service.get_subject(target_account_id)
        else {
            return Err(ModerationError::AccountNotFound);
        };

        if !self
            .silence_policy
            .has_permissions(&executing_account.role, &target_account.role)
        {
            return Err(ModerationError::InsufficientPermissions);
        }

        self.player_repository
            .set_player_silenced(target_player_id, true);
        Ok(())
    }
}
