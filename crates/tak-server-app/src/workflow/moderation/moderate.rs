use std::sync::Arc;

use crate::{
    PortsCollection,
    ports::{
        authentication::AccountPort,
        email::EmailPort,
        moderation::ModerationRepository,
        transaction::{RepoError, TransactionManager},
    },
};
use tak_domain::{
    UserId,
    moderation::{Moderation, PermissionPolicy, UserRole},
};

#[async_trait::async_trait]
pub trait ModeratePlayerUseCase {
    async fn get_moderation(&self, user_id: UserId) -> Result<Moderation, ModerationError>;
    async fn ban_player(
        &self,
        user_id: UserId,
        target_user_id: UserId,
        reason: &str,
    ) -> Result<(), ModerationError>;
    async fn unban_player(
        &self,
        user_id: UserId,
        target_user_id: UserId,
    ) -> Result<(), ModerationError>;
    async fn silence_player(
        &self,
        user_id: UserId,
        target_user_id: UserId,
    ) -> Result<(), ModerationError>;
    async fn unsilence_player(
        &self,
        user_id: UserId,
        target_user_id: UserId,
    ) -> Result<(), ModerationError>;
    async fn set_moderator(
        &self,
        user_id: UserId,
        target_user_id: UserId,
    ) -> Result<(), ModerationError>;
    async fn set_user(
        &self,
        user_id: UserId,
        target_user_id: UserId,
    ) -> Result<(), ModerationError>;
    async fn set_admin(
        &self,
        user_id: UserId,
        target_user_id: UserId,
    ) -> Result<(), ModerationError>;
}

pub enum ModerationError {
    UserNotFound,
    InsufficientPermissions,
    Internal,
}

pub struct ModerationPolicies {
    pub ban_policy: Arc<dyn PermissionPolicy + Send + Sync + 'static>,
    pub silence_policy: Arc<dyn PermissionPolicy + Send + Sync + 'static>,
    pub set_moderator_policy: Arc<dyn PermissionPolicy + Send + Sync + 'static>,
    pub set_admin_policy: Arc<dyn PermissionPolicy + Send + Sync + 'static>,
    pub set_user_policy: Arc<dyn PermissionPolicy + Send + Sync + 'static>,
}

pub struct ModeratePlayerUseCaseImpl<Ports: PortsCollection, E: EmailPort, A: AccountPort> {
    ports: Arc<Ports>,
    email_port: Arc<E>,
    policies: ModerationPolicies,
    account_info_service: Arc<A>,
}

impl<
    Ports: PortsCollection + Send + Sync + 'static,
    E: EmailPort + Send + Sync + 'static,
    A: AccountPort + Send + Sync + 'static,
> ModeratePlayerUseCaseImpl<Ports, E, A>
{
    pub fn new(
        ports: Arc<Ports>,
        email_port: Arc<E>,
        policies: ModerationPolicies,
        account_info_service: Arc<A>,
    ) -> Self {
        Self {
            ports,
            email_port,
            policies,
            account_info_service,
        }
    }

    async fn do_moderation(
        &self,
        user_id: UserId,
        target_user_id: UserId,
        policy: Arc<dyn PermissionPolicy + Send + Sync + 'static>,
        task: impl FnOnce(&mut Moderation),
    ) -> Result<(), ModerationError> {
        let ctx = self
            .ports
            .transaction_manager()
            .transaction()
            .await
            .map_err(|RepoError(e)| {
                tracing::error!("Failed to start transaction: {:?}", e);
                ModerationError::Internal
            })?;
        let executing_account = match self
            .ports
            .moderation_repository()
            .get_moderation(&ctx, user_id)
            .await
        {
            Ok(Some(account)) => account,
            Ok(None) => {
                return Err(ModerationError::UserNotFound);
            }
            Err(RepoError(e)) => {
                tracing::error!("Failed to retrieve account for user {}: {}", user_id, e);
                return Err(ModerationError::Internal);
            }
        };
        let mut target_account = match self
            .ports
            .moderation_repository()
            .get_moderation(&ctx, target_user_id)
            .await
        {
            Ok(Some(account)) => account,
            Ok(None) => {
                tracing::error!("Target user {} not found", target_user_id);
                return Err(ModerationError::UserNotFound);
            }
            Err(RepoError(e)) => {
                tracing::error!(
                    "Failed to retrieve account for user {}: {}",
                    target_user_id,
                    e
                );
                return Err(ModerationError::Internal);
            }
        };

        if !policy.has_permissions(&executing_account, &target_account) {
            return Err(ModerationError::InsufficientPermissions);
        }

        task(&mut target_account);

        self.ports
            .moderation_repository()
            .set_moderation(&ctx, target_user_id, target_account)
            .await
            .map_err(|RepoError(e)| {
                tracing::error!(
                    "Failed to update moderation for account {}: {}",
                    target_user_id,
                    e
                );
                ModerationError::Internal
            })?;

        self.ports
            .transaction_manager()
            .commit(ctx)
            .await
            .map_err(|RepoError(e)| {
                tracing::error!("Failed to commit transaction: {:?}", e);
                ModerationError::Internal
            })?;
        Ok(())
    }

    async fn set_player_banned(
        &self,
        user_id: UserId,
        target_user_id: UserId,
        is_banned: bool,
    ) -> Result<(), ModerationError> {
        self.do_moderation(
            user_id,
            target_user_id,
            self.policies.ban_policy.clone(),
            |target_account| {
                target_account.set_banned(is_banned);
            },
        )
        .await
    }

    async fn set_player_silenced(
        &self,
        user_id: UserId,
        target_user_id: UserId,
        is_silenced: bool,
    ) -> Result<(), ModerationError> {
        self.do_moderation(
            user_id,
            target_user_id,
            self.policies.silence_policy.clone(),
            |target_account| {
                target_account.set_silenced(is_silenced);
            },
        )
        .await
    }

    async fn set_role(
        &self,
        user_id: UserId,
        target_user_id: UserId,
        role: UserRole,
    ) -> Result<(), ModerationError> {
        let policy = match role {
            UserRole::Moderator => &self.policies.set_moderator_policy,
            UserRole::Admin => &self.policies.set_admin_policy,
            UserRole::User => &self.policies.set_user_policy,
        };

        self.do_moderation(user_id, target_user_id, policy.clone(), |target_account| {
            target_account.set_role(role);
        })
        .await
    }
}

#[async_trait::async_trait]
impl<
    Ports: PortsCollection + Send + Sync + 'static,
    E: EmailPort + Send + Sync + 'static,
    A: AccountPort + Send + Sync + 'static,
> ModeratePlayerUseCase for ModeratePlayerUseCaseImpl<Ports, E, A>
{
    async fn get_moderation(&self, user_id: UserId) -> Result<Moderation, ModerationError> {
        let ctx = self.ports.transaction_manager().simple();
        match self
            .ports
            .moderation_repository()
            .get_moderation(&ctx, user_id)
            .await
        {
            Ok(Some(moderation)) => Ok(moderation),
            Ok(None) => Err(ModerationError::UserNotFound),
            Err(RepoError(e)) => {
                tracing::error!(
                    "Failed to retrieve moderation for user_id: {}: {:?}",
                    user_id,
                    e
                );
                Err(ModerationError::Internal)
            }
        }
    }

    async fn ban_player(
        &self,
        user_id: UserId,
        target_user_id: UserId,
        reason: &str,
    ) -> Result<(), ModerationError> {
        self.set_player_banned(user_id, target_user_id, true)
            .await?;

        let target_account_info = match self
            .account_info_service
            .get_account_info(target_user_id)
            .await
        {
            Ok(Some(info)) => info,
            Ok(None) => return Err(ModerationError::Internal),
            Err(()) => {
                tracing::error!("Failed to get account info for user_id: {}", target_user_id);
                return Err(ModerationError::Internal);
            }
        };

        if let Some(email) = &target_account_info.email {
            let subject = "Playtak Account Banned";
            let body = format!(
                "Hello {},\n\n\
        Your account has been banned for the following reason:\n\
        {}\n\n\
        If you believe this is a mistake, please contact support.\n\n\
        Best regards,\n\
        The Playtak Team",
                target_account_info.username, reason
            );
            match self.email_port.send_email(&email, &subject, &body) {
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("Failed to send ban email to {}: {:?}", email, e);
                }
            }
        }

        Ok(())
    }

    async fn unban_player(
        &self,
        user_id: UserId,
        target_user_id: UserId,
    ) -> Result<(), ModerationError> {
        self.set_player_banned(user_id, target_user_id, false)
            .await?;
        Ok(())
    }

    async fn silence_player(
        &self,
        user_id: UserId,
        target_user_id: UserId,
    ) -> Result<(), ModerationError> {
        self.set_player_silenced(user_id, target_user_id, true)
            .await
    }

    async fn unsilence_player(
        &self,
        user_id: UserId,
        target_user_id: UserId,
    ) -> Result<(), ModerationError> {
        self.set_player_silenced(user_id, target_user_id, false)
            .await
    }

    async fn set_moderator(
        &self,
        user_id: UserId,
        target_user_id: UserId,
    ) -> Result<(), ModerationError> {
        self.set_role(user_id, target_user_id, UserRole::Moderator)
            .await
    }

    async fn set_user(
        &self,
        user_id: UserId,
        target_user_id: UserId,
    ) -> Result<(), ModerationError> {
        self.set_role(user_id, target_user_id, UserRole::User).await
    }

    async fn set_admin(
        &self,
        user_id: UserId,
        target_user_id: UserId,
    ) -> Result<(), ModerationError> {
        self.set_role(user_id, target_user_id, UserRole::Admin)
            .await
    }
}
