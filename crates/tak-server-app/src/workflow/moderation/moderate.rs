use std::sync::Arc;

use crate::ports::{authentication::AccountPort, email::EmailPort};
use tak_domain::{
    RepoError, RepoRetrieveError, UserId,
    moderation::{Moderation, ModerationRepository, PermissionPolicy, UserRole},
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

pub struct ModeratePlayerUseCaseImpl<E: EmailPort, M: ModerationRepository, A: AccountPort> {
    email_port: Arc<E>,
    policies: ModerationPolicies,
    moderation_repo: Arc<M>,
    account_info_service: Arc<A>,
}

impl<
    E: EmailPort + Send + Sync + 'static,
    M: ModerationRepository + Send + Sync + 'static,
    A: AccountPort + Send + Sync + 'static,
> ModeratePlayerUseCaseImpl<E, M, A>
{
    pub fn new(
        email_port: Arc<E>,
        policies: ModerationPolicies,
        moderation_repo: Arc<M>,
        account_info_service: Arc<A>,
    ) -> Self {
        Self {
            email_port,
            policies,
            moderation_repo,
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
        let executing_account = self.get_moderation(user_id).await?;
        let mut target_account = self.get_moderation(target_user_id).await?;

        if !policy.has_permissions(&executing_account, &target_account) {
            return Err(ModerationError::InsufficientPermissions);
        }

        task(&mut target_account);

        match self
            .moderation_repo
            .set_moderation(target_user_id, target_account)
            .await
        {
            Ok(()) => Ok(()),
            Err(RepoError::StorageError(e)) => {
                tracing::error!("Failed to set moderation: {}", e);
                Err(ModerationError::Internal)
            }
        }
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
    E: EmailPort + Send + Sync + 'static,
    M: ModerationRepository + Send + Sync + 'static,
    A: AccountPort + Send + Sync + 'static,
> ModeratePlayerUseCase for ModeratePlayerUseCaseImpl<E, M, A>
{
    async fn get_moderation(&self, user_id: UserId) -> Result<Moderation, ModerationError> {
        match self.moderation_repo.get_moderation(user_id).await {
            Ok(moderation) => Ok(moderation),
            Err(RepoRetrieveError::NotFound) => Err(ModerationError::UserNotFound),
            Err(RepoRetrieveError::StorageError(e)) => {
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
