use std::{sync::Arc, time::SystemTime};

use tak_domain::{GameId, game::GameCheckTimeoutError};

use crate::workflow::gameplay::{ObserveOutcome, lifecycle::LifecycleGameWorkflow};

pub trait GameTimeoutRunner {
    fn schedule_game_timeout_check(this: Arc<Self>, game_id: GameId);
}

pub struct GameTimeoutRunnerImpl<RpCtx, L: LifecycleGameWorkflow<RpCtx>> {
    ports: std::marker::PhantomData<RpCtx>,
    lifecycle_game_workflow: Arc<L>,
}

impl<RpCtx: Send + Sync + 'static, L: LifecycleGameWorkflow<RpCtx> + Send + Sync + 'static>
    GameTimeoutRunner for GameTimeoutRunnerImpl<RpCtx, L>
{
    fn schedule_game_timeout_check(this: Arc<Self>, game_id: GameId) {
        tokio::spawn(async move {
            Self::run(this, game_id).await;
        });
    }
}

impl<RpCtx: Send + Sync + 'static, L: LifecycleGameWorkflow<RpCtx> + Send + Sync + 'static>
    GameTimeoutRunnerImpl<RpCtx, L>
{
    pub fn new(lifecycle_game_workflow: Arc<L>) -> Self {
        Self {
            ports: std::marker::PhantomData,
            lifecycle_game_workflow,
        }
    }

    async fn run(this: Arc<Self>, game_id: GameId) {
        loop {
            let mut result = ObserveOutcome::Finished;
            match this
                .lifecycle_game_workflow
                .modify_game(game_id, |game, _| {
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
                Ok(Err(())) => {
                    tracing::error!(
                        "Error checking game timeout for game {:?}: Game not found",
                        game_id
                    );
                    return;
                }
                Err(()) => {
                    tracing::error!(
                        "Error checking game timeout for game {:?}: Internal error",
                        game_id
                    );
                    return;
                }
            };
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
