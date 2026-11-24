use std::{sync::Arc, time::SystemTime};

use tak_domain::{GameId, game::GameCheckTimeoutError};

use crate::{
    PortsCollection, application::gameplay::ObserveOutcome, workflow::lifecycle::modify_game,
};

pub trait GameTimeoutRunner {
    fn schedule_game_timeout_check(this: Arc<Self>, game_id: GameId);
}

pub struct GameTimeoutRunnerImpl<Ports: PortsCollection> {
    ports: Arc<Ports>,
}

impl<Ports: PortsCollection + Send + Sync + 'static> GameTimeoutRunner
    for GameTimeoutRunnerImpl<Ports>
{
    fn schedule_game_timeout_check(this: Arc<Self>, game_id: GameId) {
        tokio::spawn(async move {
            Self::run(this, game_id).await;
        });
    }
}

impl<Ports: PortsCollection + Send + Sync + 'static> GameTimeoutRunnerImpl<Ports> {
    pub fn new(ports: Arc<Ports>) -> Self {
        Self { ports }
    }

    async fn run(this: Arc<Self>, game_id: GameId) {
        loop {
            let mut result = ObserveOutcome::Finished;
            let ctx = match this.ports.transaction_manager().transaction().await {
                Ok(ctx) => ctx,
                Err(e) => {
                    tracing::error!("Error starting transaction for game {:?}: {:?}", game_id, e);
                    return;
                }
            };
            match modify_game(this.ports.as_ref(), &ctx, game_id, |game, _| {
                let now = SystemTime::now();
                match game.check_timeout(now) {
                    Err(GameCheckTimeoutError::Timeout) => Ok(()),
                    Ok(time_info) => {
                        result = ObserveOutcome::Continue(
                            time_info.white_remaining.min(time_info.black_remaining)
                                + std::time::Duration::from_millis(100),
                        );
                        Ok(())
                    }
                }
            })
            .await
            {
                Ok(Ok(())) => {}
                Ok(Err(())) => {}
                Err(e) => {
                    tracing::error!(
                        "Error checking game timeout for game {:?}: {:?}",
                        game_id,
                        e
                    );
                    return;
                }
            };
            match this.ports.transaction_manager().commit(ctx).await {
                Ok(_) => {}
                Err(e) => {
                    tracing::error!(
                        "Error committing transaction for game {:?}: {:?}",
                        game_id,
                        e
                    );
                    return;
                }
            }
            match result {
                ObserveOutcome::Finished => {
                    tracing::info!("Game {:?} timeout processing finished", game_id);
                    return;
                }
                ObserveOutcome::Continue(delay) => {
                    tracing::info!(
                        "Scheduling next timeout check for game {:?} in {:?}",
                        game_id,
                        delay
                    );
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }
}
