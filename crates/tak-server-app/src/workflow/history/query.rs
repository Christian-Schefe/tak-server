use crate::{
    PortsCollection,
    ports::{
        PaginatedResponse,
        game_history::{GameQuery, GameRepository},
        transaction::{TransactionError, TransactionManager},
    },
    workflow::history::GameRecordView,
};
use std::sync::Arc;
use tak_domain::{GameId, MatchId};

#[async_trait::async_trait]
pub trait GameHistoryQueryUseCase {
    async fn get_game(&self, game_id: GameId) -> Result<Option<GameRecordView>, GameQueryError>;
    async fn query_games(
        &self,
        filter: GameQuery,
    ) -> Result<PaginatedResponse<GameRecordView>, GameQueryError>;
    async fn get_games_of_match(
        &self,
        match_id: MatchId,
    ) -> Result<Vec<GameRecordView>, GameQueryError>;
}

pub enum GameQueryError {
    RepositoryError,
}

pub struct GameHistoryQueryUseCaseImpl<Ports: PortsCollection> {
    ports: Arc<Ports>,
}

impl<Ports: PortsCollection> GameHistoryQueryUseCaseImpl<Ports> {
    pub fn new(ports: Arc<Ports>) -> Self {
        Self { ports }
    }
}

#[async_trait::async_trait]
impl<Ports: PortsCollection + Send + Sync + 'static> GameHistoryQueryUseCase
    for GameHistoryQueryUseCaseImpl<Ports>
{
    async fn query_games(
        &self,
        filter: GameQuery,
    ) -> Result<PaginatedResponse<GameRecordView>, GameQueryError> {
        match self
            .ports
            .transaction_manager()
            .execute(self.ports.game_repository().query_games(filter))
            .await
        {
            Ok(result) => {
                Ok(result.map(|(id, record)| GameRecordView::from_game_record(id, record)))
            }
            Err(TransactionError(e)) => {
                tracing::error!("Error querying games: {}", e);
                Err(GameQueryError::RepositoryError)
            }
        }
    }

    async fn get_game(&self, game_id: GameId) -> Result<Option<GameRecordView>, GameQueryError> {
        match self
            .ports
            .transaction_manager()
            .execute(self.ports.game_repository().get_game_record(game_id))
            .await
        {
            Ok(Some(result)) => Ok(Some(GameRecordView::from_game_record(game_id, result))),
            Ok(None) => Ok(None),
            Err(TransactionError(e)) => {
                tracing::error!("Error getting game record: {}", e);
                Err(GameQueryError::RepositoryError)
            }
        }
    }

    async fn get_games_of_match(
        &self,
        match_id: MatchId,
    ) -> Result<Vec<GameRecordView>, GameQueryError> {
        match self
            .ports
            .transaction_manager()
            .execute(self.ports.game_repository().get_games_of_match(match_id))
            .await
        {
            Ok(game_records) => Ok(game_records
                .into_iter()
                .map(|(id, record)| GameRecordView::from_game_record(id, record))
                .collect()),
            Err(TransactionError(e)) => {
                tracing::error!("Error getting games of match {}: {}", match_id, e);
                Err(GameQueryError::RepositoryError)
            }
        }
    }
}
