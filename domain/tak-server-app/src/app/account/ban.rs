use std::sync::Arc;

use crate::{
    domain::account::AccountRepository,
    ports::{authentication::ClientId, contact::ContactRepository, email::EmailPort},
};

pub trait BanAccountUseCase {
    fn ban_account(&self, username: &str, reason: &str) -> Result<(), BanAccountError>;
}

pub enum BanAccountError {
    AccountNotFound,
    InsufficientPermissions,
}

pub struct BanAccountUseCaseImpl<A: AccountRepository, C: ContactRepository, E: EmailPort> {
    account_repository: Arc<A>,
    contact_repository: Arc<C>,
    email_port: Arc<E>,
}

impl<A: AccountRepository, C: ContactRepository, E: EmailPort> BanAccountUseCaseImpl<A, C, E> {
    pub fn new(account_repository: Arc<A>, contact_repository: Arc<C>, email_port: Arc<E>) -> Self {
        Self {
            account_repository,
            contact_repository,
            email_port,
        }
    }
}

impl<A: AccountRepository, C: ContactRepository, E: EmailPort> BanAccountUseCase
    for BanAccountUseCaseImpl<A, C, E>
{
    fn ban_account(&self, username: &str, reason: &str) -> Result<(), BanAccountError> {
        let client_id = ClientId::new(username);
        let Some(account) = self.account_repository.get_account_by_username(username) else {
            return Err(BanAccountError::AccountNotFound);
        };
        self.account_repository
            .set_player_banned(account.player_id, true);

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
        if let Some(email) = self.contact_repository.get_email_contact(&client_id) {
            match self.email_port.send_email(&email, &subject, &body) {
                Ok(_) => {}
                Err(_) => {} // Ok if email fails, we still ban the account
            }
        }
        Ok(())
    }
}
