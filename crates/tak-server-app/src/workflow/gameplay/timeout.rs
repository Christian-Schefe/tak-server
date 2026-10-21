use crate::{
    PortsCollection,
    ports::{
        game::GameRepository,
        transaction::{RepoError, TransactionManager},
    },
    workflow::gameplay::finalize_game::FinalizeGameWorkflow,
};
use std::{sync::Arc, time::Instant};
use tak_domain::{
    GameId, UserId,
    game::{CheckDisconnectTimeoutResult, CheckTimeoutResult, Game, GamePlayerActionResult},
};

pub enum ObserveOutcome {
    Finished,
    Continue(std::time::Duration),
}

pub enum ObserveError {
    GameNotFound,
    Internal,
}

#[async_trait::async_trait]
pub trait ObserveGameTimeoutUseCase {
    async fn check_game_timeout(&self, game_id: GameId) -> Result<ObserveOutcome, ObserveError>;
    async fn check_player_timeout(
        &self,
        user_id: UserId,
        disconnected_at: Instant,
    ) -> Result<ObserveOutcome, ObserveError>;
}

pub struct ObserveGameTimeoutUseCaseImpl<
    Ports: PortsCollection,
    F: FinalizeGameWorkflow<Ports::TrCtx>,
> {
    ports: Arc<Ports>,
    finalize_game_workflow: Arc<F>,
}

impl<Ports: PortsCollection, F: FinalizeGameWorkflow<Ports::TrCtx>>
    ObserveGameTimeoutUseCaseImpl<Ports, F>
{
    pub fn new(ports: Arc<Ports>, finalize_game_workflow: Arc<F>) -> Self {
        Self {
            ports,
            finalize_game_workflow,
        }
    }
}

#[async_trait::async_trait]
impl<
    Ports: PortsCollection + Send + Sync + 'static,
    F: FinalizeGameWorkflow<Ports::TrCtx> + Send + Sync + 'static,
> ObserveGameTimeoutUseCase for ObserveGameTimeoutUseCaseImpl<Ports, F>
{
    async fn check_game_timeout(&self, game_id: GameId) -> Result<ObserveOutcome, ObserveError> {
        let now = Instant::now();
        let ctx = self.ports.transaction_manager().simple();
        let mut game = match self.ports.game_repository().get_game(&ctx, game_id).await {
            Ok(Some(Game::Ongoing(game))) => game,
            Ok(Some(Game::Finished(_))) => {
                return Ok(ObserveOutcome::Finished);
            }
            Ok(None) => return Err(ObserveError::GameNotFound),
            Err(RepoError(e)) => {
                tracing::error!("Error retrieving game {}: {}", game_id, e);
                return Err(ObserveError::Internal);
            }
        };
        match game.check_timeout(now) {
            CheckTimeoutResult::TimedOut(finished_game) => {
                tracing::info!("Game {:?} timed out, finalizing game", game_id);
                if let Err(()) = self
                    .finalize_game_workflow
                    .finalize_game(&ctx, game_id, game, finished_game)
                    .await
                {
                    return Err(ObserveError::Internal);
                }
                Ok(ObserveOutcome::Finished)
            }
            CheckTimeoutResult::NoTimeout(time_info) => Ok(ObserveOutcome::Continue(
                time_info.white_remaining.min(time_info.black_remaining)
                    + std::time::Duration::from_millis(100),
            )),
        }
    }

    async fn check_player_timeout(
        &self,
        user_id: UserId,
        disconnected_at: Instant,
    ) -> Result<ObserveOutcome, ObserveError> {
        let now = Instant::now();
        let disconnected_duration = now.saturating_duration_since(disconnected_at);
        let ctx = self.ports.transaction_manager().simple();
        let games = match self
            .ports
            .game_repository()
            .get_games_by_player(&ctx, user_id)
            .await
        {
            Ok(games) => games
                .into_iter()
                .filter_map(|(game_id, game_record)| {
                    if let Game::Ongoing(game) = game_record {
                        Some((game_id, game))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>(),
            Err(RepoError(e)) => {
                tracing::error!("Error retrieving games for player {}: {}", user_id, e);
                return Err(ObserveError::Internal);
            }
        };

        let mut wait_duration = None;

        for (game_id, mut game) in games {
            match game.check_disconnect_timeout(user_id, disconnected_duration, now) {
                GamePlayerActionResult::NotAPlayerInGame => {
                    tracing::warn!(
                        "Received unexpected result when checking disconnect timeout for player {:?} in game {:?}",
                        user_id,
                        game_id
                    );
                }
                GamePlayerActionResult::Timeout(ended_game) => {
                    let _ = self
                        .finalize_game_workflow
                        .finalize_game(&ctx, game_id, game, ended_game)
                        .await;
                }
                GamePlayerActionResult::Result(res) => match res {
                    CheckDisconnectTimeoutResult::TimedOut(ended_game) => {
                        tracing::info!(
                            "Player {:?} timed out in game {:?} after being disconnected for {:?}",
                            user_id,
                            game_id,
                            disconnected_duration
                        );
                        if let Err(()) = self
                            .finalize_game_workflow
                            .finalize_game(&ctx, game_id, game, ended_game)
                            .await
                        {
                            tracing::error!(
                                "Failed to finalize game {:?} after player {:?} timed out",
                                game_id,
                                user_id
                            );
                            return Err(ObserveError::Internal);
                        }
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
            Some(duration) => Ok(ObserveOutcome::Continue(
                duration + std::time::Duration::from_millis(100),
            )),
            None => Ok(ObserveOutcome::Finished),
        }
    }
}
