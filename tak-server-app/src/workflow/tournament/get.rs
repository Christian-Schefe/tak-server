use std::sync::Arc;

use crate::{
    domain::{
        RepoError, RepoRetrieveError, TournamentId,
        tournament::{TournamentPlayerRegistrationRepository, TournamentRepository},
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
    TPR: TournamentPlayerRegistrationRepository,
> {
    tournament_repository: Arc<TR>,
    player_registration_repository: Arc<TPR>,
}

impl<TR: TournamentRepository, TPR: TournamentPlayerRegistrationRepository>
    GetTournamentUseCaseImpl<TR, TPR>
{
    pub fn new(tournament_repository: Arc<TR>, player_registration_repository: Arc<TPR>) -> Self {
        Self {
            tournament_repository,
            player_registration_repository,
        }
    }
}

#[async_trait::async_trait]
impl<
    TR: TournamentRepository + Send + Sync + 'static,
    TPR: TournamentPlayerRegistrationRepository + Send + Sync + 'static,
> GetTournamentUseCase for GetTournamentUseCaseImpl<TR, TPR>
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
            self.player_registration_repository
                .get_registered_players(tournament_id)
        ) {
            (Ok(tournament), Ok(registered_players)) => {
                Ok(Some(TournamentDetailView::from_tournament(
                    tournament_id,
                    tournament,
                    registered_players,
                )))
            }
            (Err(RepoRetrieveError::NotFound), _) => Ok(None),
            (Err(RepoRetrieveError::StorageError(e)), _) | (_, Err(RepoError::StorageError(e))) => {
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
