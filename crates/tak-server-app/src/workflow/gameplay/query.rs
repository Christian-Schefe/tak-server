use crate::{
    PortsCollection,
    ports::{
        PaginatedResponse,
        game::{GameQuery, GameRepository},
        transaction::{RepoError, TransactionManager},
    },
};
use std::sync::Arc;
use tak_domain::{GameId, MatchId, game_record::GameRecord};

#[async_trait::async_trait]
pub trait GameQueryUseCase {
    async fn get_game(&self, game_id: GameId) -> Result<Option<GameRecord>, GameQueryError>;
    async fn query_games(
        &self,
        filter: GameQuery,
    ) -> Result<PaginatedResponse<(GameId, GameRecord)>, GameQueryError>;
    async fn get_games_of_match(
        &self,
        match_id: MatchId,
    ) -> Result<Vec<(GameId, GameRecord)>, GameQueryError>;
    async fn get_ongoing_games(&self) -> Result<Vec<(GameId, GameRecord)>, GameQueryError>;
}

pub enum GameQueryError {
    RepositoryError,
}

pub struct GameQueryUseCaseImpl<Ports: PortsCollection> {
    ports: Arc<Ports>,
}

impl<Ports: PortsCollection> GameQueryUseCaseImpl<Ports> {
    pub fn new(ports: Arc<Ports>) -> Self {
        Self { ports }
    }
}

#[async_trait::async_trait]
impl<Ports: PortsCollection + Send + Sync + 'static> GameQueryUseCase
    for GameQueryUseCaseImpl<Ports>
{
    async fn query_games(
        &self,
        filter: GameQuery,
    ) -> Result<PaginatedResponse<(GameId, GameRecord)>, GameQueryError> {
        let ctx = self.ports.transaction_manager().simple();
        match self.ports.game_repository().query_games(&ctx, filter).await {
            Ok(result) => Ok(result.map(|(id, record)| (id, record))),
            Err(RepoError(e)) => {
                tracing::error!("Error querying games: {}", e);
                Err(GameQueryError::RepositoryError)
            }
        }
    }

    async fn get_game(&self, game_id: GameId) -> Result<Option<GameRecord>, GameQueryError> {
        let ctx = self.ports.transaction_manager().simple();
        let game = match self.ports.game_repository().get_game(&ctx, game_id).await {
            Ok(Some(result)) => result,
            Ok(None) => return Ok(None),
            Err(RepoError(e)) => {
                tracing::error!("Error getting game record: {}", e);
                return Err(GameQueryError::RepositoryError);
            }
        };
        Ok(Some(game))
    }

    async fn get_games_of_match(
        &self,
        match_id: MatchId,
    ) -> Result<Vec<(GameId, GameRecord)>, GameQueryError> {
        let ctx = self.ports.transaction_manager().simple();
        match self
            .ports
            .game_repository()
            .get_games_of_match(&ctx, match_id)
            .await
        {
            Ok(game_records) => Ok(game_records),
            Err(RepoError(e)) => {
                tracing::error!("Error getting games of match {}: {}", match_id, e);
                Err(GameQueryError::RepositoryError)
            }
        }
    }

    async fn get_ongoing_games(&self) -> Result<Vec<(GameId, GameRecord)>, GameQueryError> {
        let ctx = self.ports.transaction_manager().simple();
        match self.ports.game_repository().get_ongoing_games(&ctx).await {
            Ok(ongoing_games) => Ok(ongoing_games),
            Err(RepoError(e)) => {
                tracing::error!("Error getting ongoing games: {}", e);
                Err(GameQueryError::RepositoryError)
            }
        }
    }
}
