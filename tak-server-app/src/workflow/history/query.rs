use std::sync::Arc;

use crate::domain::{
    FinishedGameId, PaginatedResponse, RepoError, RepoRetrieveError,
    game_history::{GameQuery, GameRecord, GameRepository},
};

#[async_trait::async_trait]
pub trait GameHistoryQueryUseCase {
    async fn get_game(&self, game_id: FinishedGameId)
    -> Result<Option<GameRecord>, GameQueryError>;
    async fn query_games(
        &self,
        filter: GameQuery,
    ) -> Result<PaginatedResponse<(FinishedGameId, GameRecord)>, GameQueryError>;
}

pub enum GameQueryError {
    RepositoryError,
}

pub struct GameHistoryQueryUseCaseImpl<G: GameRepository> {
    game_repository: Arc<G>,
}

impl<G: GameRepository> GameHistoryQueryUseCaseImpl<G> {
    pub fn new(game_repository: Arc<G>) -> Self {
        Self { game_repository }
    }
}

#[async_trait::async_trait]
impl<G: GameRepository + Send + Sync + 'static> GameHistoryQueryUseCase
    for GameHistoryQueryUseCaseImpl<G>
{
    async fn query_games(
        &self,
        filter: GameQuery,
    ) -> Result<PaginatedResponse<(FinishedGameId, GameRecord)>, GameQueryError> {
        match self.game_repository.query_games(filter).await {
            Ok(result) => Ok(result),
            Err(RepoError::StorageError(e)) => {
                log::error!("Error querying games: {}", e);
                Err(GameQueryError::RepositoryError)
            }
        }
    }
    async fn get_game(
        &self,
        game_id: FinishedGameId,
    ) -> Result<Option<GameRecord>, GameQueryError> {
        match self.game_repository.get_game_record(game_id).await {
            Ok(result) => Ok(Some(result)),
            Err(RepoRetrieveError::StorageError(e)) => {
                log::error!("Error getting game record: {}", e);
                Err(GameQueryError::RepositoryError)
            }
            Err(RepoRetrieveError::NotFound) => Ok(None),
        }
    }
}
