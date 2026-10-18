use std::sync::Arc;

use crate::{
    PortsCollection,
    ports::{
        RepoError, RepoRetrieveError,
        tournament::{TournamentPlayerRepository, TournamentRepository, TournamentRoundRepository},
        transaction::{TransactionError, TransactionManager},
    },
    workflow::tournament::{TournamentDetailView, TournamentView},
};
use tak_domain::TournamentId;

#[async_trait::async_trait]
pub trait GetTournamentUseCase {
    async fn get_tournaments(&self) -> Result<Vec<TournamentView>, ()>;
    async fn get_tournament(
        &self,
        tournament_id: TournamentId,
    ) -> Result<Option<TournamentDetailView>, ()>;
}

pub struct GetTournamentUseCaseImpl<Ports: PortsCollection> {
    ports: Arc<Ports>,
}

impl<Ports: PortsCollection> GetTournamentUseCaseImpl<Ports> {
    pub fn new(ports: Arc<Ports>) -> Self {
        Self { ports }
    }
}

#[async_trait::async_trait]
impl<Ports: PortsCollection + Send + Sync + 'static> GetTournamentUseCase
    for GetTournamentUseCaseImpl<Ports>
{
    async fn get_tournaments(&self) -> Result<Vec<TournamentView>, ()> {
        match self.ports.tournament_repository().list_tournaments().await {
            Ok(tournaments) => Ok(tournaments
                .into_iter()
                .map(|(id, t)| TournamentView::from_tournament(id, t))
                .collect()),
            Err(RepoError::StorageError(e)) => {
                tracing::error!("Failed to list tournaments: {:?}", e);
                Err(())
            }
        }
    }

    async fn get_tournament(
        &self,
        tournament_id: TournamentId,
    ) -> Result<Option<TournamentDetailView>, ()> {
        let tournament_repo = self.ports.tournament_repository();
        let transaction_manager = self.ports.transaction_manager();
        let tournament_player_repo = self.ports.tournament_player_repository();
        let tournament_round_repo = self.ports.tournament_round_repository();
        match futures::join!(
            tournament_repo.get_tournament(tournament_id),
            transaction_manager.execute_without_transaction(
                tournament_player_repo.get_tournament_players(tournament_id)
            ),
            tournament_round_repo.get_tournament_rounds(tournament_id)
        ) {
            (Ok(tournament), Ok(tournament_players), Ok(rounds)) => {
                Ok(Some(TournamentDetailView::from_tournament(
                    tournament_id,
                    tournament,
                    tournament_players,
                    rounds,
                )))
            }
            (Err(RepoRetrieveError::NotFound), _, _) => Ok(None),
            (Err(RepoRetrieveError::StorageError(e)), _, _)
            | (_, Err(TransactionError(e)), _)
            | (_, _, Err(RepoError::StorageError(e))) => {
                tracing::error!(
                    "Failed to get tournament with id {}: {:?}",
                    tournament_id,
                    e
                );
                Err(())
            }
        }
    }
}
