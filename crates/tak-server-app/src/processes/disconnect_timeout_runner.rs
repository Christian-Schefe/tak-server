use std::{sync::Arc, time::Instant};

use dashmap::DashMap;
use tak_domain::UserId;
use tokio::select;
use tokio_util::sync::CancellationToken;

use crate::workflow::gameplay::timeout::{ObserveError, ObserveGameTimeoutUseCase, ObserveOutcome};

pub trait DisconnectTimeoutRunner {
    fn start_disconnect_timeout(this: Arc<Self>, user_id: UserId);
    fn cancel_disconnect_timeout(&self, user_id: UserId);
}

pub struct DisconnectTimeoutRunnerImpl<O: ObserveGameTimeoutUseCase + Send + Sync + 'static> {
    tasks: Arc<DashMap<UserId, CancellationToken>>,
    observer: Arc<O>,
}

impl<O: ObserveGameTimeoutUseCase + Send + Sync + 'static> DisconnectTimeoutRunnerImpl<O> {
    pub fn new(observer: Arc<O>) -> Self {
        Self {
            tasks: Arc::new(DashMap::new()),
            observer,
        }
    }

    async fn run(
        this: Arc<Self>,
        user_id: UserId,
        disconnected_at: Instant,
        token: CancellationToken,
    ) {
        loop {
            if token.is_cancelled() {
                tracing::info!("Disconnect timeout cancelled for player {:?}", user_id);
                return;
            }
            let res = this
                .observer
                .check_player_timeout(user_id, disconnected_at)
                .await;
            match res {
                Ok(ObserveOutcome::Finished) => {
                    tracing::info!(
                        "Player {:?} disconnect timeout processing finished",
                        user_id
                    );
                    return;
                }
                Ok(ObserveOutcome::Continue(delay)) => {
                    tracing::info!(
                        "Scheduling next timeout check for player {:?} in {:?}",
                        user_id,
                        delay
                    );
                    select! {
                        _ = token.cancelled() => {
                            tracing::info!("Disconnect timeout cancelled for player {:?}", user_id);
                            return;
                        }
                        _ = tokio::time::sleep(delay) => {}
                    }
                }
                Err(ObserveError::GameNotFound) => {
                    tracing::error!(
                        "Error checking disconnect timeout for player {:?}: Game not found",
                        user_id
                    );
                    return;
                }
                Err(ObserveError::Internal) => {
                    tracing::error!(
                        "Error checking disconnect timeout for player {:?}: Internal error",
                        user_id
                    );
                    return;
                }
            }
        }
    }
}

impl<O: ObserveGameTimeoutUseCase + Send + Sync + 'static> DisconnectTimeoutRunner
    for DisconnectTimeoutRunnerImpl<O>
{
    fn start_disconnect_timeout(this: Arc<Self>, user_id: UserId) {
        tracing::info!("Starting disconnect timeout for player {:?}", user_id);
        let token = CancellationToken::new();
        let disconnected_at = Instant::now();
        if let Some(prev) = this.tasks.insert(user_id, token.clone()) {
            prev.cancel();
        }
        tokio::spawn(async move {
            Self::run(this, user_id, disconnected_at, token).await;
            tracing::info!(
                "Disconnect timeout task exited for player {:?} who disconnected at {:?}",
                user_id,
                disconnected_at
            );
        });
    }

    fn cancel_disconnect_timeout(&self, user_id: UserId) {
        tracing::info!("Cancelling disconnect timeout for player {:?}", user_id);
        if let Some((_, prev)) = self.tasks.remove(&user_id) {
            prev.cancel();
        }
    }
}
