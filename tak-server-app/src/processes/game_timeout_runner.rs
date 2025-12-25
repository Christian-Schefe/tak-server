use std::{sync::Arc, time::Instant};

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
            let now = Instant::now();
            match this.observer.tick(game_id, now).await {
                ObserveOutcome::Finished => return,
                ObserveOutcome::Continue(delay) => {
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }
}
