use std::sync::Arc;

use crate::{
    PortsCollection,
    ports::{
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
        match self
            .ports
            .transaction_manager()
            .execute(self.ports.tournament_repository().list_tournaments())
            .await
        {
            Ok(tournaments) => Ok(tournaments
                .into_iter()
                .map(|(id, t)| TournamentView::from_tournament(id, t))
                .collect()),
            Err(TransactionError(e)) => {
                tracing::error!("Failed to list tournaments: {:?}", e);
                Err(())
            }
        }
    }

    async fn get_tournament(
        &self,
        tournament_id: TournamentId,
    ) -> Result<Option<TournamentDetailView>, ()> {
        match self
            .ports
            .transaction_manager()
            .execute(
                self.ports
                    .tournament_repository()
                    .get_tournament(tournament_id)
                    .try_simultaneous(
                        self.ports
                            .tournament_player_repository()
                            .get_tournament_players(tournament_id),
                    )
                    .try_simultaneous(
                        self.ports
                            .tournament_round_repository()
                            .get_tournament_rounds(tournament_id),
                    ),
            )
            .await
        {
            Ok(((tournament, tournament_players), rounds)) => {
                let Some(tournament) = tournament else {
                    return Ok(None);
                };
                Ok(Some(TournamentDetailView::from_tournament(
                    tournament_id,
                    tournament,
                    tournament_players,
                    rounds,
                )))
            }
            Err(TransactionError(e)) => {
                tracing::error!("Failed to get tournament: {:?}", e);
                Err(())
            }
        }
    }
}
