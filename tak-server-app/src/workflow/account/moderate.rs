use std::sync::Arc;

use crate::{
    domain::{
        AccountId,
        moderation::{AccountRole, ModerationFlag, PermissionPolicy},
    },
    ports::{
        authentication::{Account, AuthenticationPort},
        email::EmailPort,
    },
};

#[async_trait::async_trait]
pub trait ModeratePlayerUseCase {
    async fn kick_player(
        &self,
        account_id: &AccountId,
        target_account_id: &AccountId,
    ) -> Result<(), ModerationError>;
    async fn ban_player(
        &self,
        account_id: &AccountId,
        target_account_id: &AccountId,
        reason: &str,
    ) -> Result<(), ModerationError>;
    async fn unban_player(
        &self,
        account_id: &AccountId,
        target_account_id: &AccountId,
    ) -> Result<(), ModerationError>;
    async fn silence_player(
        &self,
        account_id: &AccountId,
        target_account_id: &AccountId,
    ) -> Result<(), ModerationError>;
    async fn unsilence_player(
        &self,
        account_id: &AccountId,
        target_account_id: &AccountId,
    ) -> Result<(), ModerationError>;
    async fn set_moderator(
        &self,
        account_id: &AccountId,
        target_account_id: &AccountId,
    ) -> Result<(), ModerationError>;
    async fn set_user(
        &self,
        account_id: &AccountId,
        target_account_id: &AccountId,
    ) -> Result<(), ModerationError>;
    async fn set_admin(
        &self,
        account_id: &AccountId,
        target_account_id: &AccountId,
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

pub struct ModeratePlayerUseCaseImpl<E: EmailPort, A: AuthenticationPort> {
    email_port: Arc<E>,
    policies: ModerationPolicies,
    authentication_service: Arc<A>,
}

impl<E: EmailPort, A: AuthenticationPort> ModeratePlayerUseCaseImpl<E, A> {
    pub fn new(
        email_port: Arc<E>,
        policies: ModerationPolicies,
        authentication_service: Arc<A>,
    ) -> Self {
        Self {
            email_port,
            policies,
            authentication_service,
        }
    }

    async fn get_account(&self, account_id: &AccountId) -> Result<Account, ModerationError> {
        if let Some(account) = self.authentication_service.get_account(account_id).await {
            Ok(account)
        } else {
            Err(ModerationError::AccountNotFound)
        }
    }

    async fn set_player_banned(
        &self,
        account_id: &AccountId,
        target_account_id: &AccountId,
        is_banned: bool,
    ) -> Result<Account, ModerationError> {
        let executing_account = self.get_account(account_id).await?;
        let target_account = self.get_account(target_account_id).await?;

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
        account_id: &AccountId,
        target_account_id: &AccountId,
        is_silenced: bool,
    ) -> Result<(), ModerationError> {
        let executing_account = self.get_account(account_id).await?;
        let target_account = self.get_account(target_account_id).await?;

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
        account_id: &AccountId,
        target_account_id: &AccountId,
        role: AccountRole,
    ) -> Result<(), ModerationError> {
        let executing_account = self.get_account(account_id).await?;
        let target_account = self.get_account(target_account_id).await?;

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
impl<E: EmailPort + Send + Sync + 'static, A: AuthenticationPort + Send + Sync + 'static>
    ModeratePlayerUseCase for ModeratePlayerUseCaseImpl<E, A>
{
    async fn kick_player(
        &self,
        account_id: &AccountId,
        target_account_id: &AccountId,
    ) -> Result<(), ModerationError> {
        let executing_account = self.get_account(account_id).await?;
        let target_account = self.get_account(target_account_id).await?;

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
        account_id: &AccountId,
        target_account_id: &AccountId,
        reason: &str,
    ) -> Result<(), ModerationError> {
        let target_account = self
            .set_player_banned(account_id, target_account_id, true)
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
                    log::error!("Failed to send ban email to {}: {:?}", email, e);
                }
            }
        }

        Ok(())
    }

    async fn unban_player(
        &self,
        account_id: &AccountId,
        target_account_id: &AccountId,
    ) -> Result<(), ModerationError> {
        self.set_player_banned(account_id, target_account_id, false)
            .await?;
        Ok(())
    }

    async fn silence_player(
        &self,
        account_id: &AccountId,
        target_account_id: &AccountId,
    ) -> Result<(), ModerationError> {
        self.set_player_silenced(account_id, target_account_id, true)
            .await
    }

    async fn unsilence_player(
        &self,
        account_id: &AccountId,
        target_account_id: &AccountId,
    ) -> Result<(), ModerationError> {
        self.set_player_silenced(account_id, target_account_id, false)
            .await
    }

    async fn set_moderator(
        &self,
        account_id: &AccountId,
        target_account_id: &AccountId,
    ) -> Result<(), ModerationError> {
        self.set_role(account_id, target_account_id, AccountRole::Moderator)
            .await
    }

    async fn set_user(
        &self,
        account_id: &AccountId,
        target_account_id: &AccountId,
    ) -> Result<(), ModerationError> {
        self.set_role(account_id, target_account_id, AccountRole::User)
            .await
    }

    async fn set_admin(
        &self,
        account_id: &AccountId,
        target_account_id: &AccountId,
    ) -> Result<(), ModerationError> {
        self.set_role(account_id, target_account_id, AccountRole::Admin)
            .await
    }
}
