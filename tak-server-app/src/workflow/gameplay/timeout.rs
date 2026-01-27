use std::{sync::Arc, time::Instant};

use crate::{
    domain::{
        GameId,
        game::{CheckTimeoutResult, GameService},
    },
    workflow::gameplay::finalize_game::FinalizeGameWorkflow,
};

pub enum ObserveOutcome {
    Finished,
    Continue(std::time::Duration),
}

#[async_trait::async_trait]
pub trait ObserveGameTimeoutUseCase {
    async fn tick(&self, game_id: GameId) -> ObserveOutcome;
}

pub struct ObserveGameTimeoutUseCaseImpl<G: GameService, F: FinalizeGameWorkflow> {
    game_service: Arc<G>,
    finalize_game_workflow: Arc<F>,
}

impl<G: GameService, F: FinalizeGameWorkflow> ObserveGameTimeoutUseCaseImpl<G, F> {
    pub fn new(game_service: Arc<G>, finalize_game_workflow: Arc<F>) -> Self {
        Self {
            game_service,
            finalize_game_workflow,
        }
    }
}

#[async_trait::async_trait]
impl<G: GameService + Send + Sync + 'static, F: FinalizeGameWorkflow + Send + Sync + 'static>
    ObserveGameTimeoutUseCase for ObserveGameTimeoutUseCaseImpl<G, F>
{
    async fn tick(&self, game_id: GameId) -> ObserveOutcome {
        let now = Instant::now();
        match self.game_service.check_timeout(game_id, now) {
            CheckTimeoutResult::TimedOut(game) => {
                self.finalize_game_workflow.finalize_game(game).await;
                ObserveOutcome::Finished
            }
            CheckTimeoutResult::NoTimeout(time_info) => ObserveOutcome::Continue(
                time_info.white_remaining.min(time_info.black_remaining)
                    + std::time::Duration::from_millis(100),
            ),

            CheckTimeoutResult::GameNotFound => ObserveOutcome::Finished,
        }
    }
}
