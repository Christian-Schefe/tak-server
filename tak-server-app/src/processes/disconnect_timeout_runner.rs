use std::{sync::Arc, time::Instant};

use dashmap::DashMap;
use tokio::select;
use tokio_util::sync::CancellationToken;

use crate::{
    domain::PlayerId,
    workflow::gameplay::timeout::{ObserveGameTimeoutUseCase, ObserveOutcome},
};

pub trait DisconnectTimeoutRunner {
    fn start_disconnect_timeout(this: Arc<Self>, player_id: PlayerId);
    fn cancel_disconnect_timeout(&self, player_id: PlayerId);
}

pub struct DisconnectTimeoutRunnerImpl<O: ObserveGameTimeoutUseCase + Send + Sync + 'static> {
    tasks: Arc<DashMap<PlayerId, CancellationToken>>,
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
        player_id: PlayerId,
        disconnected_at: Instant,
        token: CancellationToken,
    ) {
        loop {
            match select! {
                _ = token.cancelled() => {
                    log::info!("Disconnect timeout cancelled for player {:?}", player_id);
                    return;
                }
                res = this.observer.check_player_timeout(player_id, disconnected_at) => {res}
            } {
                ObserveOutcome::Finished => {
                    log::info!(
                        "Player {:?} disconnect timeout processing finished",
                        player_id
                    );
                    return;
                }
                ObserveOutcome::Continue(delay) => {
                    log::info!(
                        "Scheduling next timeout check for player {:?} in {:?}",
                        player_id,
                        delay
                    );
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }
}

impl<O: ObserveGameTimeoutUseCase + Send + Sync + 'static> DisconnectTimeoutRunner
    for DisconnectTimeoutRunnerImpl<O>
{
    fn start_disconnect_timeout(this: Arc<Self>, player_id: PlayerId) {
        log::info!("Starting disconnect timeout for player {:?}", player_id);
        let token = CancellationToken::new();
        let disconnected_at = Instant::now();
        if let Some(prev) = this.tasks.insert(player_id, token.clone()) {
            prev.cancel();
        }
        tokio::spawn(async move {
            Self::run(this, player_id, disconnected_at, token).await;
        });
    }

    fn cancel_disconnect_timeout(&self, player_id: PlayerId) {
        log::info!("Cancelling disconnect timeout for player {:?}", player_id);
        if let Some((_, prev)) = self.tasks.remove(&player_id) {
            prev.cancel();
        }
    }
}
