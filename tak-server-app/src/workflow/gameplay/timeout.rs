use std::{sync::Arc, time::Instant};

use crate::{
    domain::{
        GameId, PlayerId,
        game::{
            CheckDisconnectTimeoutResult, CheckTimeoutResult, GamePlayerActionResult, GameService,
        },
    },
    workflow::gameplay::finalize_game::FinalizeGameWorkflow,
};

pub enum ObserveOutcome {
    Finished,
    Continue(std::time::Duration),
}

#[async_trait::async_trait]
pub trait ObserveGameTimeoutUseCase {
    async fn check_game_timeout(&self, game_id: GameId) -> ObserveOutcome;
    async fn check_player_timeout(
        &self,
        player_id: PlayerId,
        disconnected_at: Instant,
    ) -> ObserveOutcome;
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
    async fn check_game_timeout(&self, game_id: GameId) -> ObserveOutcome {
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

    async fn check_player_timeout(
        &self,
        player_id: PlayerId,
        disconnected_at: Instant,
    ) -> ObserveOutcome {
        let now = Instant::now();
        let disconnected_since = now.saturating_duration_since(disconnected_at);
        let games = self
            .game_service
            .get_games()
            .filter(|game| {
                game.metadata.white_id == player_id || game.metadata.black_id == player_id
            })
            .collect::<Vec<_>>();

        let mut wait_duration = None;

        for game in games {
            match self.game_service.check_disconnect_timeout(
                game.metadata.game_id,
                player_id,
                disconnected_since,
                now,
            ) {
                GamePlayerActionResult::GameNotFound | GamePlayerActionResult::NotAPlayerInGame => {
                    log::warn!(
                        "Received unexpected result when checking disconnect timeout for player {:?} in game {:?}",
                        player_id,
                        game.metadata.game_id
                    );
                }
                GamePlayerActionResult::Timeout(ended_game) => {
                    self.finalize_game_workflow.finalize_game(ended_game).await;
                }
                GamePlayerActionResult::Result(res) => match res {
                    CheckDisconnectTimeoutResult::TimedOut(ended_game) => {
                        self.finalize_game_workflow.finalize_game(ended_game).await;
                    }
                    CheckDisconnectTimeoutResult::CantTimeOut => {}
                    CheckDisconnectTimeoutResult::NoTimeout(duration) => {
                        wait_duration = match wait_duration {
                            Some(current) if current < duration => Some(current),
                            _ => Some(duration),
                        };
                    }
                },
            }
        }

        match wait_duration {
            Some(duration) => {
                ObserveOutcome::Continue(duration + std::time::Duration::from_millis(100))
            }
            None => ObserveOutcome::Finished,
        }
    }
}
