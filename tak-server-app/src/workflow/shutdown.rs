use std::{sync::Arc, time::Instant};

use crate::{domain::game::GameService, workflow::gameplay::finalize_game::FinalizeGameWorkflow};

#[async_trait::async_trait]
pub trait ShutdownWorkflow {
    async fn shutdown(&self);
}

pub struct ShutdownWorkflowImpl<F: FinalizeGameWorkflow, G: GameService> {
    finalize_game_workflow: Arc<F>,
    game_service: Arc<G>,
}

impl<F: FinalizeGameWorkflow, G: GameService> ShutdownWorkflowImpl<F, G> {
    pub fn new(finalize_game_workflow: Arc<F>, game_service: Arc<G>) -> Self {
        Self {
            finalize_game_workflow,
            game_service,
        }
    }
}

#[async_trait::async_trait]
impl<F: FinalizeGameWorkflow + Send + Sync + 'static, G: GameService + Send + Sync + 'static>
    ShutdownWorkflow for ShutdownWorkflowImpl<F, G>
{
    async fn shutdown(&self) {
        let games = self.game_service.abort_all_games(Instant::now());
        let finalize_futures = games
            .into_iter()
            .map(|game| self.finalize_game_workflow.finalize_game(game));
        futures::future::join_all(finalize_futures).await;
    }
}
