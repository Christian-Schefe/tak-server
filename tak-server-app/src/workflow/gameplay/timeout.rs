use std::sync::Arc;

use tak_core::{TakInstant, TakTimeInfo};

use crate::{
    domain::{
        GameId,
        game::{CheckTimoutResult, GameService},
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
        let now = TakInstant::now();
        match self.game_service.check_timeout(game_id, now) {
            CheckTimoutResult::GameTimedOut(game) => {
                self.finalize_game_workflow.finalize_game(game).await;
                ObserveOutcome::Finished
            }
            CheckTimoutResult::NoTimeout(remaining) => ObserveOutcome::Continue(match remaining {
                TakTimeInfo::Realtime {
                    white_remaining,
                    black_remaining,
                } => white_remaining.min(black_remaining) + std::time::Duration::from_millis(100),
                TakTimeInfo::Async { next_deadline } => {
                    let until_deadline_ms = next_deadline
                        .signed_duration_since(now.async_time)
                        .num_milliseconds();
                    if until_deadline_ms <= 0 {
                        std::time::Duration::from_secs(5 * 60)
                    } else {
                        std::time::Duration::from_millis(until_deadline_ms as u64 + 5 * 60 * 1000)
                    }
                }
            }),
            CheckTimoutResult::GameNotFound => ObserveOutcome::Finished,
        }
    }
}
