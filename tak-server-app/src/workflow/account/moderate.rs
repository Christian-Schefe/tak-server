use std::sync::Arc;

use crate::{
    domain::{
        PlayerId,
        account::{AccountFlag, AccountRole, PermissionPolicy},
        player::PlayerRepository,
    },
    ports::{
        authentication::{Account, AuthSubject, AuthenticationPort},
        email::EmailPort,
    },
};

#[async_trait::async_trait]
pub trait ModeratePlayerUseCase {
    async fn kick_player(
        &self,
        player_id: PlayerId,
        target_player_id: PlayerId,
    ) -> Result<(), ModerationError>;
    async fn ban_player(
        &self,
        player_id: PlayerId,
        target_player_id: PlayerId,
        reason: &str,
    ) -> Result<(), ModerationError>;
    async fn unban_player(
        &self,
        player_id: PlayerId,
        target_player_id: PlayerId,
    ) -> Result<(), ModerationError>;
    async fn silence_player(
        &self,
        player_id: PlayerId,
        target_player_id: PlayerId,
    ) -> Result<(), ModerationError>;
    async fn unsilence_player(
        &self,
        player_id: PlayerId,
        target_player_id: PlayerId,
    ) -> Result<(), ModerationError>;
    async fn set_bot(
        &self,
        player_id: PlayerId,
        target_player_id: PlayerId,
        is_bot: bool,
    ) -> Result<(), ModerationError>;
    async fn set_moderator(
        &self,
        player_id: PlayerId,
        target_player_id: PlayerId,
    ) -> Result<(), ModerationError>;
    async fn set_user(
        &self,
        player_id: PlayerId,
        target_player_id: PlayerId,
    ) -> Result<(), ModerationError>;
    async fn set_admin(
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

pub struct ModeratePlayerUseCaseImpl<E: EmailPort, PR: PlayerRepository, A: AuthenticationPort> {
    email_port: Arc<E>,
    kick_policy: Arc<dyn PermissionPolicy + Send + Sync + 'static>,
    ban_policy: Arc<dyn PermissionPolicy + Send + Sync + 'static>,
    silence_policy: Arc<dyn PermissionPolicy + Send + Sync + 'static>,
    set_bot_policy: Arc<dyn PermissionPolicy + Send + Sync + 'static>,
    set_moderator_policy: Arc<dyn PermissionPolicy + Send + Sync + 'static>,
    set_admin_policy: Arc<dyn PermissionPolicy + Send + Sync + 'static>,
    set_user_policy: Arc<dyn PermissionPolicy + Send + Sync + 'static>,
    player_repository: Arc<PR>,
    authentication_service: Arc<A>,
}

impl<E: EmailPort, PR: PlayerRepository, A: AuthenticationPort>
    ModeratePlayerUseCaseImpl<E, PR, A>
{
    pub fn new(
        email_port: Arc<E>,
        kick_policy: Arc<dyn PermissionPolicy + Send + Sync + 'static>,
        ban_policy: Arc<dyn PermissionPolicy + Send + Sync + 'static>,
        silence_policy: Arc<dyn PermissionPolicy + Send + Sync + 'static>,
        set_bot_policy: Arc<dyn PermissionPolicy + Send + Sync + 'static>,
        set_moderator_policy: Arc<dyn PermissionPolicy + Send + Sync + 'static>,
        set_admin_policy: Arc<dyn PermissionPolicy + Send + Sync + 'static>,
        set_user_policy: Arc<dyn PermissionPolicy + Send + Sync + 'static>,
        player_repository: Arc<PR>,
        authentication_service: Arc<A>,
    ) -> Self {
        Self {
            email_port,
            kick_policy,
            ban_policy,
            silence_policy,
            set_bot_policy,
            set_moderator_policy,
            set_admin_policy,
            set_user_policy,
            player_repository,
            authentication_service,
        }
    }

    async fn get_account(&self, player_id: PlayerId) -> Result<Account, ModerationError> {
        let account_id = match self.player_repository.get_player(player_id).await {
            Ok(player) => player.account_id,
            _ => return Err(ModerationError::PlayerNotFound),
        };
        if let Some(account_id) = account_id
            && let Some(account) = self.authentication_service.get_account(account_id).await
        {
            Ok(account)
        } else {
            Err(ModerationError::AccountNotFound)
        }
    }

    async fn set_player_banned(
        &self,
        player_id: PlayerId,
        target_player_id: PlayerId,
        is_banned: bool,
    ) -> Result<Account, ModerationError> {
        let executing_account = self.get_account(player_id).await?;
        let target_account = self.get_account(target_player_id).await?;

        if !self
            .ban_policy
            .has_permissions(&executing_account.role, &target_account.role)
        {
            return Err(ModerationError::InsufficientPermissions);
        }

        let res = if is_banned {
            self.authentication_service
                .add_flag(target_account.account_id, AccountFlag::Banned)
                .await
        } else {
            self.authentication_service
                .remove_flag(target_account.account_id, AccountFlag::Banned)
                .await
        };

        if let Err(_) = res {
            log::error!("Failed to set player banned status");
            return Err(ModerationError::PlayerNotFound);
        }

        Ok(target_account)
    }

    async fn set_player_silenced(
        &self,
        player_id: PlayerId,
        target_player_id: PlayerId,
        is_silenced: bool,
    ) -> Result<(), ModerationError> {
        let executing_account = self.get_account(player_id).await?;
        let target_account = self.get_account(target_player_id).await?;

        if !self
            .silence_policy
            .has_permissions(&executing_account.role, &target_account.role)
        {
            return Err(ModerationError::InsufficientPermissions);
        }

        let res = if is_silenced {
            self.authentication_service
                .add_flag(target_account.account_id, AccountFlag::Silenced)
                .await
        } else {
            self.authentication_service
                .remove_flag(target_account.account_id, AccountFlag::Silenced)
                .await
        };

        if let Err(_) = res {
            log::error!("Failed to set player silenced status");
            return Err(ModerationError::PlayerNotFound);
        }

        Ok(())
    }

    async fn set_role(
        &self,
        player_id: PlayerId,
        target_player_id: PlayerId,
        role: AccountRole,
    ) -> Result<(), ModerationError> {
        let executing_account = self.get_account(player_id).await?;
        let target_account = self.get_account(target_player_id).await?;

        let policy = match role {
            AccountRole::Moderator => &self.set_moderator_policy,
            AccountRole::Admin => &self.set_admin_policy,
            AccountRole::User => &self.set_user_policy,
        };

        if !policy.has_permissions(&executing_account.role, &target_account.role) {
            return Err(ModerationError::InsufficientPermissions);
        }

        match self
            .authentication_service
            .set_role(target_account.account_id, AccountRole::Moderator)
            .await
        {
            Ok(()) => Ok(()),
            Err(()) => {
                log::error!("Failed to set player role to {:?}", role);
                Err(ModerationError::PlayerNotFound)
            }
        }
    }
}

#[async_trait::async_trait]
impl<
    E: EmailPort + Send + Sync + 'static,
    PR: PlayerRepository + Send + Sync + 'static,
    A: AuthenticationPort + Send + Sync + 'static,
> ModeratePlayerUseCase for ModeratePlayerUseCaseImpl<E, PR, A>
{
    async fn kick_player(
        &self,
        player_id: PlayerId,
        target_player_id: PlayerId,
    ) -> Result<(), ModerationError> {
        let executing_account = self.get_account(player_id).await?;
        let target_account = self.get_account(target_player_id).await?;

        if !self
            .kick_policy
            .has_permissions(&executing_account.role, &target_account.role)
        {
            return Err(ModerationError::InsufficientPermissions);
        }

        Ok(())
    }

    async fn ban_player(
        &self,
        player_id: PlayerId,
        target_player_id: PlayerId,
        reason: &str,
    ) -> Result<(), ModerationError> {
        let target_account = self
            .set_player_banned(player_id, target_player_id, true)
            .await?;

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

    async fn unban_player(
        &self,
        player_id: PlayerId,
        target_player_id: PlayerId,
    ) -> Result<(), ModerationError> {
        self.set_player_banned(player_id, target_player_id, false)
            .await?;
        Ok(())
    }

    async fn silence_player(
        &self,
        player_id: PlayerId,
        target_player_id: PlayerId,
    ) -> Result<(), ModerationError> {
        self.set_player_silenced(player_id, target_player_id, true)
            .await
    }

    async fn unsilence_player(
        &self,
        player_id: PlayerId,
        target_player_id: PlayerId,
    ) -> Result<(), ModerationError> {
        self.set_player_silenced(player_id, target_player_id, false)
            .await
    }

    async fn set_bot(
        &self,
        player_id: PlayerId,
        target_player_id: PlayerId,
        is_bot: bool,
    ) -> Result<(), ModerationError> {
        let executing_account = self.get_account(player_id).await?;
        let target_account = self.get_account(target_player_id).await?;

        if !self
            .set_bot_policy
            .has_permissions(&executing_account.role, &target_account.role)
        {
            return Err(ModerationError::InsufficientPermissions);
        }

        let res = if is_bot {
            self.authentication_service
                .add_flag(target_account.account_id, AccountFlag::Bot)
                .await
        } else {
            self.authentication_service
                .remove_flag(target_account.account_id, AccountFlag::Bot)
                .await
        };

        if let Err(_) = res {
            log::error!("Failed to set player bot status");
            return Err(ModerationError::PlayerNotFound);
        }

        Ok(())
    }

    async fn set_moderator(
        &self,
        player_id: PlayerId,
        target_player_id: PlayerId,
    ) -> Result<(), ModerationError> {
        self.set_role(player_id, target_player_id, AccountRole::Moderator)
            .await
    }

    async fn set_user(
        &self,
        player_id: PlayerId,
        target_player_id: PlayerId,
    ) -> Result<(), ModerationError> {
        self.set_role(player_id, target_player_id, AccountRole::User)
            .await
    }

    async fn set_admin(
        &self,
        player_id: PlayerId,
        target_player_id: PlayerId,
    ) -> Result<(), ModerationError> {
        self.set_role(player_id, target_player_id, AccountRole::Admin)
            .await
    }
}
