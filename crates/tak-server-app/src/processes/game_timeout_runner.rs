use std::sync::Arc;

use tak_domain::GameId;

use crate::workflow::gameplay::do_action::{DoActionUseCase, ObserveError, ObserveOutcome};

pub trait GameTimeoutRunner {
    fn schedule_game_timeout_check(this: Arc<Self>, game_id: GameId);
}

pub struct GameTimeoutRunnerImpl<O: DoActionUseCase + Send + Sync + 'static> {
    observer: Arc<O>,
}

impl<O: DoActionUseCase + Send + Sync + 'static> GameTimeoutRunner for GameTimeoutRunnerImpl<O> {
    fn schedule_game_timeout_check(this: Arc<Self>, game_id: GameId) {
        tokio::spawn(async move {
            Self::run(this, game_id).await;
        });
    }
}

impl<O: DoActionUseCase + Send + Sync + 'static> GameTimeoutRunnerImpl<O> {
    pub fn new(observer: Arc<O>) -> Self {
        Self { observer }
    }

    async fn run(this: Arc<Self>, game_id: GameId) {
        loop {
            match this.observer.check_game_timeout(game_id).await {
                Ok(ObserveOutcome::Finished) => {
                    tracing::info!("Game {:?} timeout processing finished", game_id);
                    return;
                }
                Ok(ObserveOutcome::Continue(delay)) => {
                    tracing::info!(
                        "Scheduling next timeout check for game {:?} in {:?}",
                        game_id,
                        delay
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(ObserveError::GameNotFound) => {
                    tracing::error!(
                        "Error checking game timeout for game {:?}: Game not found",
                        game_id
                    );
                    return;
                }
                Err(ObserveError::Internal) => {
                    tracing::error!(
                        "Error checking game timeout for game {:?}: Internal error",
                        game_id
                    );
                    return;
                }
            }
        }
    }
}
