use std::sync::Arc;

use crate::{
    domain::{
        PlayerId,
        moderation::{AccountRole, ModerationFlag, PermissionPolicy},
    },
    ports::{
        authentication::{Account, AuthenticationPort},
        email::EmailPort,
        player_mapping::PlayerAccountMappingRepository,
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
    AccountNotFound,
    InsufficientPermissions,
}

pub struct ModerationPolicies {
    pub kick_policy: Arc<dyn PermissionPolicy + Send + Sync + 'static>,
    pub ban_policy: Arc<dyn PermissionPolicy + Send + Sync + 'static>,
    pub silence_policy: Arc<dyn PermissionPolicy + Send + Sync + 'static>,
    pub set_moderator_policy: Arc<dyn PermissionPolicy + Send + Sync + 'static>,
    pub set_admin_policy: Arc<dyn PermissionPolicy + Send + Sync + 'static>,
    pub set_user_policy: Arc<dyn PermissionPolicy + Send + Sync + 'static>,
}

pub struct ModeratePlayerUseCaseImpl<
    E: EmailPort,
    PR: PlayerAccountMappingRepository,
    A: AuthenticationPort,
> {
    email_port: Arc<E>,
    policies: ModerationPolicies,
    player_account_mapping_repository: Arc<PR>,
    authentication_service: Arc<A>,
}

impl<E: EmailPort, PR: PlayerAccountMappingRepository, A: AuthenticationPort>
    ModeratePlayerUseCaseImpl<E, PR, A>
{
    pub fn new(
        email_port: Arc<E>,
        policies: ModerationPolicies,
        player_account_mapping_repository: Arc<PR>,
        authentication_service: Arc<A>,
    ) -> Self {
        Self {
            email_port,
            policies,
            player_account_mapping_repository,
            authentication_service,
        }
    }

    async fn get_account(&self, player_id: PlayerId) -> Result<Account, ModerationError> {
        let account_id = match self
            .player_account_mapping_repository
            .get_account_id(player_id)
            .await
        {
            Ok(acc_id) => acc_id,
            _ => return Err(ModerationError::AccountNotFound),
        };
        if let Some(account) = self.authentication_service.get_account(&account_id).await {
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
            .policies
            .ban_policy
            .has_permissions(&executing_account.role, &target_account.role)
        {
            return Err(ModerationError::InsufficientPermissions);
        }

        let res = if is_banned {
            self.authentication_service
                .add_flag(&target_account.account_id, ModerationFlag::Banned)
                .await
        } else {
            self.authentication_service
                .remove_flag(&target_account.account_id, ModerationFlag::Banned)
                .await
        };

        if let Err(_) = res {
            log::error!("Failed to set player banned status");
            return Err(ModerationError::AccountNotFound);
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
            .policies
            .silence_policy
            .has_permissions(&executing_account.role, &target_account.role)
        {
            return Err(ModerationError::InsufficientPermissions);
        }

        let res = if is_silenced {
            self.authentication_service
                .add_flag(&target_account.account_id, ModerationFlag::Silenced)
                .await
        } else {
            self.authentication_service
                .remove_flag(&target_account.account_id, ModerationFlag::Silenced)
                .await
        };

        if let Err(_) = res {
            log::error!("Failed to set player silenced status");
            return Err(ModerationError::AccountNotFound);
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
            AccountRole::Moderator => &self.policies.set_moderator_policy,
            AccountRole::Admin => &self.policies.set_admin_policy,
            AccountRole::User => &self.policies.set_user_policy,
        };

        if !policy.has_permissions(&executing_account.role, &target_account.role) {
            return Err(ModerationError::InsufficientPermissions);
        }

        match self
            .authentication_service
            .set_role(&target_account.account_id, AccountRole::Moderator)
            .await
        {
            Ok(()) => Ok(()),
            Err(()) => {
                log::error!("Failed to set player role to {:?}", role);
                Err(ModerationError::AccountNotFound)
            }
        }
    }
}

#[async_trait::async_trait]
impl<
    E: EmailPort + Send + Sync + 'static,
    PR: PlayerAccountMappingRepository + Send + Sync + 'static,
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
            .policies
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

        if let Some(email) = &target_account.email {
            let subject = "Playtak Account Banned";
            let body = format!(
                "Hello {},\n\n\
        Your account has been banned for the following reason:\n\
        {}\n\n\
        If you believe this is a mistake, please contact support.\n\n\
        Best regards,\n\
        The Playtak Team",
                target_account.username, reason
            );
            match self.email_port.send_email(&email, &subject, &body) {
                Ok(_) => {}
                Err(e) => {
                    log::error!("Failed to send ban email to {}: {}", email, e);
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
