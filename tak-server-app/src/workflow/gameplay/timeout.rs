use std::{sync::Arc, time::Instant};

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

pub trait ObserveGameTimeoutUseCase {
    fn tick(&self, game_id: GameId, now: Instant) -> ObserveOutcome;
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

impl<G: GameService, F: FinalizeGameWorkflow> ObserveGameTimeoutUseCase
    for ObserveGameTimeoutUseCaseImpl<G, F>
{
    fn tick(&self, game_id: GameId, now: Instant) -> ObserveOutcome {
        match self.game_service.check_timeout(game_id, now) {
            CheckTimoutResult::GameTimedOut(game) => {
                self.finalize_game_workflow.finalize_game(game);
                ObserveOutcome::Finished
            }
            CheckTimoutResult::NoTimeout {
                white_remaining,
                black_remaining,
            } => ObserveOutcome::Continue(
                white_remaining.min(black_remaining) + std::time::Duration::from_millis(100),
            ),
        }
    }
}
