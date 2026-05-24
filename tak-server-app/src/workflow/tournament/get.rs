use std::sync::Arc;

use crate::{
    domain::{
        RepoError, RepoRetrieveError, TournamentId,
        tournament::{TournamentPlayerRepository, TournamentRepository, TournamentRoundRepository},
    },
    workflow::tournament::{TournamentDetailView, TournamentView},
};

#[async_trait::async_trait]
pub trait GetTournamentUseCase {
    async fn get_tournaments(&self) -> Result<Vec<TournamentView>, ()>;
    async fn get_tournament(
        &self,
        tournament_id: TournamentId,
    ) -> Result<Option<TournamentDetailView>, ()>;
}

pub struct GetTournamentUseCaseImpl<
    TR: TournamentRepository,
    TPR: TournamentPlayerRepository,
    TRR: TournamentRoundRepository,
> {
    tournament_repository: Arc<TR>,
    tournament_player_repository: Arc<TPR>,
    tournament_round_repository: Arc<TRR>,
}

impl<TR: TournamentRepository, TPR: TournamentPlayerRepository, TRR: TournamentRoundRepository>
    GetTournamentUseCaseImpl<TR, TPR, TRR>
{
    pub fn new(
        tournament_repository: Arc<TR>,
        tournament_player_repository: Arc<TPR>,
        tournament_round_repository: Arc<TRR>,
    ) -> Self {
        Self {
            tournament_repository,
            tournament_player_repository,
            tournament_round_repository,
        }
    }
}

#[async_trait::async_trait]
impl<
    TR: TournamentRepository + Send + Sync + 'static,
    TPR: TournamentPlayerRepository + Send + Sync + 'static,
    TRR: TournamentRoundRepository + Send + Sync + 'static,
> GetTournamentUseCase for GetTournamentUseCaseImpl<TR, TPR, TRR>
{
    #[tracing::instrument(skip(self))]
    async fn get_tournaments(&self) -> Result<Vec<TournamentView>, ()> {
        match self.tournament_repository.list_tournaments().await {
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

    #[tracing::instrument(skip(self))]
    async fn get_tournament(
        &self,
        tournament_id: TournamentId,
    ) -> Result<Option<TournamentDetailView>, ()> {
        match futures::join!(
            self.tournament_repository.get_tournament(tournament_id),
            self.tournament_player_repository
                .get_tournament_players(tournament_id),
            self.tournament_round_repository
                .get_tournament_rounds(tournament_id)
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
            | (_, Err(RepoError::StorageError(e)), _)
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
