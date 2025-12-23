use std::sync::Arc;

use crate::{
    domain::{
        PlayerId,
        account::{AccountRepository, PermissionPolicy},
        player::PlayerRepository,
    },
    ports::{authentication::SubjectId, contact::ContactRepository, email::EmailPort},
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
    A: AccountRepository,
    C: ContactRepository,
    E: EmailPort,
    BPP: PermissionPolicy,
    SPP: PermissionPolicy,
    PR: PlayerRepository,
> {
    account_repository: Arc<A>,
    contact_repository: Arc<C>,
    email_port: Arc<E>,
    ban_policy: Arc<BPP>,
    silence_policy: Arc<SPP>,
    player_repository: Arc<PR>,
}

impl<
    A: AccountRepository,
    C: ContactRepository,
    E: EmailPort,
    BPP: PermissionPolicy,
    SPP: PermissionPolicy,
    PR: PlayerRepository,
> ModeratePlayerUseCaseImpl<A, C, E, BPP, SPP, PR>
{
    pub fn new(
        account_repository: Arc<A>,
        contact_repository: Arc<C>,
        email_port: Arc<E>,
        ban_policy: Arc<BPP>,
        silence_policy: Arc<SPP>,
        player_repository: Arc<PR>,
    ) -> Self {
        Self {
            account_repository,
            contact_repository,
            email_port,
            ban_policy,
            silence_policy,
            player_repository,
        }
    }
}

impl<
    A: AccountRepository,
    C: ContactRepository,
    E: EmailPort,
    BPP: PermissionPolicy,
    SPP: PermissionPolicy,
    PR: PlayerRepository,
> ModeratePlayerUseCase for ModeratePlayerUseCaseImpl<A, C, E, BPP, SPP, PR>
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

        let Some(executing_account) = self.account_repository.get_account(executing_account_id)
        else {
            return Err(ModerationError::AccountNotFound);
        };
        let Some(target_account) = self.account_repository.get_account(target_account_id) else {
            return Err(ModerationError::AccountNotFound);
        };

        if !self
            .ban_policy
            .has_permissions(&executing_account, &target_account)
        {
            return Err(ModerationError::InsufficientPermissions);
        }

        self.player_repository.set_player_banned(player_id, true);

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
        let subject_id = SubjectId::Account(target_account.account_id);
        if let Some(email) = self.contact_repository.get_email_contact(&subject_id) {
            match self.email_port.send_email(&email, &subject, &body) {
                Ok(_) => {}
                Err(_) => {} // Ok if email fails, we still ban the account
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

        let Some(executing_account) = self.account_repository.get_account(executing_account_id)
        else {
            return Err(ModerationError::AccountNotFound);
        };
        let Some(target_account) = self.account_repository.get_account(target_account_id) else {
            return Err(ModerationError::AccountNotFound);
        };

        if !self
            .silence_policy
            .has_permissions(&executing_account, &target_account)
        {
            return Err(ModerationError::InsufficientPermissions);
        }

        self.player_repository
            .set_player_silenced(target_player_id, true);
        Ok(())
    }
}
