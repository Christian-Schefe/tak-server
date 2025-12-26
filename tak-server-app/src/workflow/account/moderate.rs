use std::sync::Arc;

use crate::{
    domain::{PlayerId, account::PermissionPolicy, player::PlayerRepository},
    ports::{
        authentication::{AuthContext, AuthSubject, AuthenticationService},
        email::EmailPort,
    },
};

#[async_trait::async_trait]
pub trait ModeratePlayerUseCase {
    async fn ban_player(
        &self,
        player_id: PlayerId,
        target_player_id: PlayerId,
        reason: &str,
    ) -> Result<(), ModerationError>;
    async fn silence_player(
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

    async fn get_account(&self, player_id: PlayerId) -> Result<AuthContext, ModerationError> {
        let account_id = match self.player_repository.get_player(player_id).await {
            Ok(player) => player.account_id,
            _ => return Err(ModerationError::PlayerNotFound),
        };
        if let Some(account_id) = account_id
            && let Some(account) = self.authentication_service.get_subject(account_id)
        {
            Ok(account)
        } else {
            Err(ModerationError::AccountNotFound)
        }
    }
}

#[async_trait::async_trait]
impl<
    E: EmailPort + Send + Sync + 'static,
    BPP: PermissionPolicy + Send + Sync + 'static,
    SPP: PermissionPolicy + Send + Sync + 'static,
    PR: PlayerRepository + Send + Sync + 'static,
    A: AuthenticationService + Send + Sync + 'static,
> ModeratePlayerUseCase for ModeratePlayerUseCaseImpl<E, BPP, SPP, PR, A>
{
    async fn ban_player(
        &self,
        player_id: PlayerId,
        target_player_id: PlayerId,
        reason: &str,
    ) -> Result<(), ModerationError> {
        let executing_account = self.get_account(player_id).await?;
        let target_account = self.get_account(target_player_id).await?;

        if !self
            .ban_policy
            .has_permissions(&executing_account.role, &target_account.role)
        {
            return Err(ModerationError::InsufficientPermissions);
        }

        if let Err(e) = self
            .player_repository
            .set_player_banned(player_id, true)
            .await
        {
            log::error!("Failed to ban player: {}", e);
            return Err(ModerationError::PlayerNotFound);
        }

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

    async fn silence_player(
        &self,
        player_id: PlayerId,
        target_player_id: PlayerId,
    ) -> Result<(), ModerationError> {
        let executing_account = self.get_account(player_id).await?;
        let target_account = self.get_account(target_player_id).await?;

        if !self
            .silence_policy
            .has_permissions(&executing_account.role, &target_account.role)
        {
            return Err(ModerationError::InsufficientPermissions);
        }

        match self
            .player_repository
            .set_player_silenced(target_player_id, true)
            .await
        {
            Ok(_) => {}
            Err(e) => {
                log::error!("Failed to silence player: {}", e);
                return Err(ModerationError::PlayerNotFound);
            }
        }
        Ok(())
    }
}
