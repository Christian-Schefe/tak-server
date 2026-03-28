use std::sync::Arc;

use tak_core::TakGameSettings;

use crate::domain::{
    TournamentId,
    tournament::{
        Tournament, TournamentMetadata, TournamentRepository, TournamentStatus, TournamentType,
    },
};

#[async_trait::async_trait]
pub trait HostTournamentUseCase {
    async fn create_tournament(
        &self,
        name: String,
        tournament_type: TournamentType,
        match_settings: TakGameSettings,
    ) -> Result<TournamentId, ()>;
    async fn begin_tournament(&self, tournament_id: TournamentId) -> Result<(), ()>;
}

pub struct HostTournamentUseCaseImpl<UTR: TournamentRepository> {
    tournament_repository: Arc<UTR>,
}

impl<UTR: TournamentRepository> HostTournamentUseCaseImpl<UTR> {
    pub fn new(tournament_repository: Arc<UTR>) -> Self {
        Self {
            tournament_repository,
        }
    }
}

#[async_trait::async_trait]
impl<UTR: TournamentRepository + Send + Sync + 'static> HostTournamentUseCase
    for HostTournamentUseCaseImpl<UTR>
{
    #[tracing::instrument(skip(self))]
    async fn create_tournament(
        &self,
        name: String,
        tournament_type: TournamentType,
        match_settings: TakGameSettings,
    ) -> Result<TournamentId, ()> {
        let tournament = Tournament {
            metadata: TournamentMetadata {
                name,
                tournament_type,
                match_settings,
            },
            status: TournamentStatus::Upcoming,
        };
        match self
            .tournament_repository
            .create_tournament(tournament)
            .await
        {
            Ok(tournament_id) => Ok(tournament_id),
            Err(e) => {
                tracing::error!("Failed to create tournament: {:?}", e);
                Err(())
            }
        }
    }

    #[tracing::instrument(skip(self))]
    async fn begin_tournament(&self, tournament_id: TournamentId) -> Result<(), ()> {
        let tournament = match self
            .tournament_repository
            .get_tournament(tournament_id)
            .await
        {
            Ok(tournament) => tournament,
            Err(e) => {
                tracing::error!(
                    "Failed to begin tournament with id {}: {:?}",
                    tournament_id,
                    e
                );
                return Err(());
            }
        };
        let TournamentStatus::Upcoming = tournament.status else {
            tracing::warn!(
                "Attempted to begin tournament {} which is not in Upcoming status",
                tournament_id
            );
            return Err(());
        };
        if let Err(e) = self
            .tournament_repository
            .set_tournament_status(tournament_id, TournamentStatus::Ongoing)
            .await
        {
            tracing::error!(
                "Failed to set tournament {} status to Ongoing: {:?}",
                tournament_id,
                e
            );
            return Err(());
        }
        Ok(())
    }
}
