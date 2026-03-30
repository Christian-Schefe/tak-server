use std::sync::Arc;

use crate::domain::{
    PlayerId, RepoRetrieveError, TournamentId,
    tournament::{
        TournamentPlayer, TournamentPlayerRepository, TournamentRepository, TournamentStatus,
    },
};

#[async_trait::async_trait]
pub trait TournamentPlayerRegistrationUseCase {
    async fn register_player_in_tournament(
        &self,
        tournament_id: TournamentId,
        player_id: PlayerId,
    ) -> Result<(), ()>;
    async fn unregister_player_from_tournament(
        &self,
        tournament_id: TournamentId,
        player_id: PlayerId,
    ) -> Result<(), ()>;
}

pub struct TournamentPlayerRegistrationUseCaseImpl<
    TR: TournamentRepository,
    TPR: TournamentPlayerRepository,
> {
    tournament_repository: Arc<TR>,
    tournament_player_repository: Arc<TPR>,
}

impl<TR: TournamentRepository, TPR: TournamentPlayerRepository>
    TournamentPlayerRegistrationUseCaseImpl<TR, TPR>
{
    pub fn new(tournament_repository: Arc<TR>, tournament_player_repository: Arc<TPR>) -> Self {
        Self {
            tournament_repository,
            tournament_player_repository,
        }
    }
}

#[async_trait::async_trait]
impl<
    TR: TournamentRepository + Send + Sync + 'static,
    TPR: TournamentPlayerRepository + Send + Sync + 'static,
> TournamentPlayerRegistrationUseCase for TournamentPlayerRegistrationUseCaseImpl<TR, TPR>
{
    #[tracing::instrument(skip(self))]
    async fn register_player_in_tournament(
        &self,
        tournament_id: TournamentId,
        player_id: PlayerId,
    ) -> Result<(), ()> {
        let tournament = match self
            .tournament_repository
            .get_tournament(tournament_id)
            .await
        {
            Ok(tournament) => Ok(tournament),
            Err(RepoRetrieveError::NotFound) => Err(()),
            Err(RepoRetrieveError::StorageError(e)) => {
                tracing::error!(
                    "Failed to get tournament with id {}: {:?}",
                    tournament_id,
                    e
                );
                Err(())
            }
        }?;
        let TournamentStatus::Upcoming = tournament.status else {
            tracing::warn!(
                "Attempted to register player {} in tournament {} which is not upcoming",
                player_id,
                tournament_id
            );
            return Err(());
        };
        if let Err(e) = self
            .tournament_player_repository
            .create_tournament_player(tournament_id, TournamentPlayer::new(player_id))
            .await
        {
            tracing::error!(
                "Failed to register player {} in tournament {}: {:?}",
                player_id,
                tournament_id,
                e
            );
            Err(())
        } else {
            Ok(())
        }
    }

    #[tracing::instrument(skip(self))]
    async fn unregister_player_from_tournament(
        &self,
        tournament_id: TournamentId,
        player_id: PlayerId,
    ) -> Result<(), ()> {
        let tournament = match self
            .tournament_repository
            .get_tournament(tournament_id)
            .await
        {
            Ok(tournament) => Ok(tournament),
            Err(RepoRetrieveError::NotFound) => return Ok(()),
            Err(RepoRetrieveError::StorageError(e)) => {
                tracing::error!(
                    "Failed to get tournament with id {}: {:?}",
                    tournament_id,
                    e
                );
                Err(())
            }
        }?;
        let TournamentStatus::Upcoming = tournament.status else {
            tracing::warn!(
                "Attempted to unregister player {} in tournament {} which is not upcoming",
                player_id,
                tournament_id
            );
            return Err(());
        };
        if let Err(e) = self
            .tournament_player_repository
            .remove_tournament_player(tournament_id, player_id)
            .await
        {
            tracing::error!(
                "Failed to unregister player {} from tournament {}: {:?}",
                player_id,
                tournament_id,
                e
            );
            Err(())
        } else {
            Ok(())
        }
    }
}
