use std::sync::Arc;

use crate::{
    PortsCollection,
    errors::{AppResult, InsufficientPermissions, InternalError, UserNotFound},
    ports::{
        authentication::AccountPort, email::EmailPort, moderation::ModerationRepository,
        transaction::TransactionManager,
    },
};
use tak_domain::{
    UserId,
    moderation::{
        AdminUserPolicy, HigherRoleUserPolicy, Moderation, ModeratorUserPolicy, PermissionPolicy,
        UserRole,
    },
};
use terrors::OneOf;

#[async_trait::async_trait]
pub trait ModerationUseCase {
    async fn get_moderation(
        &self,
        user_id: UserId,
    ) -> AppResult<Moderation, (UserNotFound, InternalError)>;
    async fn ban_player(
        &self,
        user_id: UserId,
        target_user_id: UserId,
        reason: &str,
    ) -> AppResult<(), (UserNotFound, InsufficientPermissions, InternalError)>;
    async fn unban_player(
        &self,
        user_id: UserId,
        target_user_id: UserId,
    ) -> AppResult<(), (UserNotFound, InsufficientPermissions, InternalError)>;
    async fn silence_player(
        &self,
        user_id: UserId,
        target_user_id: UserId,
    ) -> AppResult<(), (UserNotFound, InsufficientPermissions, InternalError)>;
    async fn unsilence_player(
        &self,
        user_id: UserId,
        target_user_id: UserId,
    ) -> AppResult<(), (UserNotFound, InsufficientPermissions, InternalError)>;
    async fn set_moderator(
        &self,
        user_id: UserId,
        target_user_id: UserId,
    ) -> AppResult<(), (UserNotFound, InsufficientPermissions, InternalError)>;
    async fn set_user(
        &self,
        user_id: UserId,
        target_user_id: UserId,
    ) -> AppResult<(), (UserNotFound, InsufficientPermissions, InternalError)>;
    async fn set_admin(
        &self,
        user_id: UserId,
        target_user_id: UserId,
    ) -> AppResult<(), (UserNotFound, InsufficientPermissions, InternalError)>;
}

pub struct ModerationUseCaseImpl<Ports: PortsCollection, E: EmailPort, A: AccountPort> {
    ports: Arc<Ports>,
    email_port: Arc<E>,
    account_info_service: Arc<A>,
}

impl<
    Ports: PortsCollection + Send + Sync + 'static,
    E: EmailPort + Send + Sync + 'static,
    A: AccountPort + Send + Sync + 'static,
> ModerationUseCaseImpl<Ports, E, A>
{
    pub fn new(ports: Arc<Ports>, email_port: Arc<E>, account_info_service: Arc<A>) -> Self {
        Self {
            ports,
            email_port,
            account_info_service,
        }
    }

    async fn do_moderation(
        &self,
        user_id: UserId,
        target_user_id: UserId,
        policy: impl PermissionPolicy,
        task: impl FnOnce(&mut Moderation),
    ) -> AppResult<(), (UserNotFound, InsufficientPermissions, InternalError)> {
        let ctx = self
            .ports
            .transaction_manager()
            .transaction()
            .await
            .map_err(OneOf::broaden)?;
        let Some(executing_account) = self
            .ports
            .moderation_repository()
            .get_moderation(&ctx, user_id)
            .await
            .map_err(OneOf::broaden)?
        else {
            return Err(OneOf::new(UserNotFound));
        };
        let Some(mut target_account) = self
            .ports
            .moderation_repository()
            .get_moderation(&ctx, target_user_id)
            .await
            .map_err(OneOf::broaden)?
        else {
            return Err(OneOf::new(UserNotFound));
        };

        if !policy.has_permissions(&executing_account, &target_account) {
            return Err(OneOf::new(InsufficientPermissions));
        }

        task(&mut target_account);

        self.ports
            .moderation_repository()
            .set_moderation(&ctx, target_user_id, target_account)
            .await
            .map_err(OneOf::broaden)?;

        self.ports
            .transaction_manager()
            .commit(ctx)
            .await
            .map_err(OneOf::broaden)?;
        Ok(())
    }

    async fn set_player_banned(
        &self,
        user_id: UserId,
        target_user_id: UserId,
        is_banned: bool,
    ) -> AppResult<(), (UserNotFound, InsufficientPermissions, InternalError)> {
        self.do_moderation(user_id, target_user_id, AdminUserPolicy, |target_account| {
            target_account.set_banned(is_banned);
        })
        .await
    }

    async fn set_player_silenced(
        &self,
        user_id: UserId,
        target_user_id: UserId,
        is_silenced: bool,
    ) -> AppResult<(), (UserNotFound, InsufficientPermissions, InternalError)> {
        self.do_moderation(
            user_id,
            target_user_id,
            ModeratorUserPolicy,
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
    ) -> AppResult<(), (UserNotFound, InsufficientPermissions, InternalError)> {
        self.do_moderation(
            user_id,
            target_user_id,
            HigherRoleUserPolicy,
            |target_account| {
                target_account.set_role(role);
            },
        )
        .await
    }
}

#[async_trait::async_trait]
impl<
    Ports: PortsCollection + Send + Sync + 'static,
    E: EmailPort + Send + Sync + 'static,
    A: AccountPort + Send + Sync + 'static,
> ModerationUseCase for ModerationUseCaseImpl<Ports, E, A>
{
    async fn get_moderation(
        &self,
        user_id: UserId,
    ) -> AppResult<Moderation, (UserNotFound, InternalError)> {
        let ctx = self.ports.transaction_manager().simple();
        let Some(moderation) = self
            .ports
            .moderation_repository()
            .get_moderation(&ctx, user_id)
            .await
            .map_err(OneOf::broaden)?
        else {
            return Err(OneOf::new(UserNotFound));
        };
        Ok(moderation)
    }

    async fn ban_player(
        &self,
        user_id: UserId,
        target_user_id: UserId,
        reason: &str,
    ) -> AppResult<(), (UserNotFound, InsufficientPermissions, InternalError)> {
        self.set_player_banned(user_id, target_user_id, true)
            .await?;

        let Some(target_account_info) = self
            .account_info_service
            .get_account_info(target_user_id)
            .await
            .map_err(OneOf::broaden)?
        else {
            return Err(OneOf::new(UserNotFound));
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
    ) -> AppResult<(), (UserNotFound, InsufficientPermissions, InternalError)> {
        self.set_player_banned(user_id, target_user_id, false)
            .await?;
        Ok(())
    }

    async fn silence_player(
        &self,
        user_id: UserId,
        target_user_id: UserId,
    ) -> AppResult<(), (UserNotFound, InsufficientPermissions, InternalError)> {
        self.set_player_silenced(user_id, target_user_id, true)
            .await
    }

    async fn unsilence_player(
        &self,
        user_id: UserId,
        target_user_id: UserId,
    ) -> AppResult<(), (UserNotFound, InsufficientPermissions, InternalError)> {
        self.set_player_silenced(user_id, target_user_id, false)
            .await
    }

    async fn set_moderator(
        &self,
        user_id: UserId,
        target_user_id: UserId,
    ) -> AppResult<(), (UserNotFound, InsufficientPermissions, InternalError)> {
        self.set_role(user_id, target_user_id, UserRole::Moderator)
            .await
    }

    async fn set_user(
        &self,
        user_id: UserId,
        target_user_id: UserId,
    ) -> AppResult<(), (UserNotFound, InsufficientPermissions, InternalError)> {
        self.set_role(user_id, target_user_id, UserRole::User).await
    }

    async fn set_admin(
        &self,
        user_id: UserId,
        target_user_id: UserId,
    ) -> AppResult<(), (UserNotFound, InsufficientPermissions, InternalError)> {
        self.set_role(user_id, target_user_id, UserRole::Admin)
            .await
    }
}
