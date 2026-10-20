use std::sync::Arc;

use tak_domain::{
    TournamentId, UserId,
    tournament::{TournamentPlayer, TournamentStatus},
};

use crate::{
    PortsCollection,
    ports::{
        tournament::{TournamentPlayerRepository, TournamentRepository},
        transaction::{RepoError, TransactionManager},
    },
};

#[async_trait::async_trait]
pub trait TournamentPlayerRegistrationUseCase {
    async fn register_player_in_tournament(
        &self,
        tournament_id: TournamentId,
        user_id: UserId,
    ) -> Result<(), TournamentRegistrationError>;
    async fn unregister_player_from_tournament(
        &self,
        tournament_id: TournamentId,
        user_id: UserId,
    ) -> Result<(), TournamentRegistrationError>;
}

pub enum TournamentRegistrationError {
    TournamentNotFound,
    TournamentNotUpcoming,
    StorageError,
}

pub struct TournamentPlayerRegistrationUseCaseImpl<Ports: PortsCollection> {
    ports: Arc<Ports>,
}

impl<Ports: PortsCollection> TournamentPlayerRegistrationUseCaseImpl<Ports> {
    pub fn new(ports: Arc<Ports>) -> Self {
        Self { ports }
    }
}

#[async_trait::async_trait]
impl<Ports: PortsCollection + Send + Sync + 'static> TournamentPlayerRegistrationUseCase
    for TournamentPlayerRegistrationUseCaseImpl<Ports>
{
    #[tracing::instrument(skip(self))]
    async fn register_player_in_tournament(
        &self,
        tournament_id: TournamentId,
        user_id: UserId,
    ) -> Result<(), TournamentRegistrationError> {
        let ctx = self.ports.transaction_manager().simple();
        let tournament = match self
            .ports
            .tournament_repository()
            .get_tournament(&ctx, tournament_id)
            .await
        {
            Ok(Some(tournament)) => Ok(tournament),
            Ok(None) => Err(TournamentRegistrationError::TournamentNotFound),
            Err(RepoError(e)) => {
                tracing::error!(
                    "Failed to get tournament with id {}: {:?}",
                    tournament_id,
                    e
                );
                Err(TournamentRegistrationError::StorageError)
            }
        }?;
        let TournamentStatus::Upcoming {
            registration_open: true,
        } = tournament.status
        else {
            tracing::warn!(
                "Attempted to register player {} in tournament {} which is not upcoming or not open for registration",
                user_id,
                tournament_id
            );
            return Err(TournamentRegistrationError::TournamentNotUpcoming);
        };
        if let Err(RepoError(e)) = self
            .ports
            .tournament_player_repository()
            .create_tournament_player(&ctx, tournament_id, TournamentPlayer::new(user_id))
            .await
        {
            tracing::error!(
                "Failed to register player {} in tournament {}: {:?}",
                user_id,
                tournament_id,
                e
            );
            Err(TournamentRegistrationError::StorageError)
        } else {
            Ok(())
        }
    }

    #[tracing::instrument(skip(self))]
    async fn unregister_player_from_tournament(
        &self,
        tournament_id: TournamentId,
        user_id: UserId,
    ) -> Result<(), TournamentRegistrationError> {
        let ctx = self.ports.transaction_manager().simple();
        let tournament = match self
            .ports
            .tournament_repository()
            .get_tournament(&ctx, tournament_id)
            .await
        {
            Ok(Some(tournament)) => Ok(tournament),
            Ok(None) => {
                return Err(TournamentRegistrationError::TournamentNotFound);
            }
            Err(RepoError(e)) => {
                tracing::error!(
                    "Failed to get tournament with id {}: {:?}",
                    tournament_id,
                    e
                );
                Err(TournamentRegistrationError::StorageError)
            }
        }?;
        let TournamentStatus::Upcoming {
            registration_open: true,
        } = tournament.status
        else {
            tracing::warn!(
                "Attempted to unregister player {} in tournament {} which is not upcoming or not open for registration",
                user_id,
                tournament_id
            );
            return Err(TournamentRegistrationError::TournamentNotUpcoming);
        };
        if let Err(RepoError(e)) = self
            .ports
            .tournament_player_repository()
            .remove_tournament_player(&ctx, tournament_id, user_id)
            .await
        {
            tracing::error!(
                "Failed to unregister player {} from tournament {}: {:?}",
                user_id,
                tournament_id,
                e
            );
            Err(TournamentRegistrationError::StorageError)
        } else {
            Ok(())
        }
    }
}
