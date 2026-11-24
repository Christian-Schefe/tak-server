use std::{
    sync::Arc,
    time::{Instant, SystemTime},
};

use better_errors::OneOf;
use dashmap::DashMap;
use tak_domain::{
    UserId,
    game::{CheckDisconnectTimeoutResult, GameCheckTimeoutError},
};
use tokio::select;
use tokio_util::sync::CancellationToken;

use crate::{
    PortsCollection,
    application::gameplay::ObserveOutcome,
    errors::{AppResult, GameNotFound, GameNotOngoing, InternalError},
    workflow::lifecycle::modify_game,
};

pub trait DisconnectTimeoutRunner {
    fn start_disconnect_timeout(this: Arc<Self>, user_id: UserId);
    fn cancel_disconnect_timeout(&self, user_id: UserId);
}

pub struct DisconnectTimeoutRunnerImpl<Ports: PortsCollection> {
    tasks: Arc<DashMap<UserId, CancellationToken>>,
    ports: Arc<Ports>,
}

impl<Ports: PortsCollection + Send + Sync + 'static> DisconnectTimeoutRunnerImpl<Ports> {
    pub fn new(ports: Arc<Ports>) -> Self {
        Self {
            tasks: Arc::new(DashMap::new()),
            ports,
        }
    }

    async fn check_player_timeout(
        &self,
        user_id: UserId,
        disconnected_at: Instant,
    ) -> AppResult<ObserveOutcome, (GameNotFound, GameNotOngoing, InternalError)> {
        let now = SystemTime::now();
        let disconnected_duration = Instant::now().saturating_duration_since(disconnected_at);
        let ctx = self.ports.transaction_manager().simple();
        let games = self
            .ports
            .game_repository()
            .get_ongoing_games_by_player(&ctx, user_id)
            .await
            .map_err(OneOf::broaden)?;

        let mut wait_duration = None;

        let ctx = self
            .ports
            .transaction_manager()
            .transaction()
            .await
            .map_err(OneOf::broaden)?;

        for (game_id, record) in games {
            let _ = modify_game(self.ports.as_ref(), &ctx, game_id, |game, _| {
                let Some(player) = record.metadata.get_player(user_id) else {
                    tracing::warn!(
                        "Player {} not found in game {} while checking player timeout",
                        user_id,
                        game_id
                    );
                    return Err(());
                };
                match game.check_disconnect_timeout(player, disconnected_duration, now) {
                    Err(GameCheckTimeoutError::Timeout) => Ok(()),
                    Ok(res) => match res {
                        CheckDisconnectTimeoutResult::TimedOut => Ok(()),
                        CheckDisconnectTimeoutResult::CantTimeOut => Ok(()),
                        CheckDisconnectTimeoutResult::NoTimeout(duration) => {
                            wait_duration = match wait_duration {
                                Some(current) if current < duration => Some(current),
                                _ => Some(duration),
                            };
                            Ok(())
                        }
                    },
                }
            })
            .await?;
        }

        self.ports
            .transaction_manager()
            .commit(ctx)
            .await
            .map_err(OneOf::broaden)?;

        match wait_duration {
            Some(duration) => Ok(ObserveOutcome::Continue(
                duration + std::time::Duration::from_millis(100),
            )),
            None => Ok(ObserveOutcome::Finished),
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
            let res = this.check_player_timeout(user_id, disconnected_at).await;
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
                Err(e) => {
                    tracing::error!(
                        "Error checking disconnect timeout for player {:?}: {:?}",
                        user_id,
                        e
                    );
                    return;
                }
            }
        }
    }
}

impl<Ports: PortsCollection + Send + Sync + 'static> DisconnectTimeoutRunner
    for DisconnectTimeoutRunnerImpl<Ports>
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
