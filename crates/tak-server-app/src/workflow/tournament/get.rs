use std::sync::Arc;

use crate::{
    PortsCollection,
    ports::{
        tournament::{TournamentPlayerRepository, TournamentRepository, TournamentRoundRepository},
        transaction::{RepoError, TransactionManager},
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
        let ctx = self.ports.transaction_manager().simple();
        match self
            .ports
            .tournament_repository()
            .list_tournaments(&ctx)
            .await
        {
            Ok(tournaments) => Ok(tournaments
                .into_iter()
                .map(|(id, t)| TournamentView::from_tournament(id, t))
                .collect()),
            Err(RepoError(e)) => {
                tracing::error!("Failed to list tournaments: {:?}", e);
                Err(())
            }
        }
    }

    async fn get_tournament(
        &self,
        tournament_id: TournamentId,
    ) -> Result<Option<TournamentDetailView>, ()> {
        let ctx = self.ports.transaction_manager().simple();
        let tournament_repo = self.ports.tournament_repository();
        let tournament_player_repo = self.ports.tournament_player_repository();
        let tournament_round_repo = self.ports.tournament_round_repository();
        match futures::try_join!(
            tournament_repo.get_tournament(&ctx, tournament_id),
            tournament_player_repo.get_tournament_players(&ctx, tournament_id),
            tournament_round_repo.get_tournament_rounds(&ctx, tournament_id),
        ) {
            Ok((tournament, tournament_players, rounds)) => {
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
            Err(RepoError(e)) => {
                tracing::error!("Failed to get tournament: {:?}", e);
                Err(())
            }
        }
    }
}
