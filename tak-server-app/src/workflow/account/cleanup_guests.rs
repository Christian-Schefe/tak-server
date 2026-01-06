use std::{sync::Arc, time::Duration};

use crate::{
    ports::authentication::AuthenticationPort,
    workflow::account::remove_account::RemoveAccountWorkflow,
};

pub struct GuestCleanupJob<A: AuthenticationPort, R: RemoveAccountWorkflow> {
    auth_port: Arc<A>,
    remove_account_workflow: Arc<R>,
}

impl<
    A: AuthenticationPort + Send + Sync + 'static,
    R: RemoveAccountWorkflow + Send + Sync + 'static,
> GuestCleanupJob<A, R>
{
    pub fn new(auth_port: Arc<A>, remove_account_workflow: Arc<R>) -> Self {
        Self {
            auth_port,
            remove_account_workflow,
        }
    }

    pub async fn run(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(60 * 60));
        loop {
            let removed_guests = self.auth_port.clean_up_guest_accounts().await;
            for guest_id in removed_guests {
                if let Err(e) = self.remove_account_workflow.remove_account(&guest_id).await {
                    log::error!("Failed to remove guest account {}: {:?}", guest_id, e);
                }
            }
            interval.tick().await;
        }
    }
}
