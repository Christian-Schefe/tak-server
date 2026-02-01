use std::sync::Arc;

use crate::{
    domain::GameId,
    workflow::gameplay::timeout::{ObserveGameTimeoutUseCase, ObserveOutcome},
};

pub trait GameTimeoutRunner {
    fn schedule_game_timeout_check(this: Arc<Self>, game_id: GameId);
}

pub struct GameTimeoutRunnerImpl<O: ObserveGameTimeoutUseCase + Send + Sync + 'static> {
    observer: Arc<O>,
}

impl<O: ObserveGameTimeoutUseCase + Send + Sync + 'static> GameTimeoutRunner
    for GameTimeoutRunnerImpl<O>
{
    fn schedule_game_timeout_check(this: Arc<Self>, game_id: GameId) {
        tokio::spawn(async move {
            Self::run(this, game_id).await;
        });
    }
}

impl<O: ObserveGameTimeoutUseCase + Send + Sync + 'static> GameTimeoutRunnerImpl<O> {
    pub fn new(observer: Arc<O>) -> Self {
        Self { observer }
    }

    async fn run(this: Arc<Self>, game_id: GameId) {
        loop {
            match this.observer.check_game_timeout(game_id).await {
                ObserveOutcome::Finished => {
                    log::info!("Game {:?} timeout processing finished", game_id);
                    return;
                }
                ObserveOutcome::Continue(delay) => {
                    log::info!(
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
