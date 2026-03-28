use tak_core::TakGameSettings;

use crate::domain::{PlayerId, RepoError, RepoRetrieveError, TournamentId};

#[derive(Clone, Debug)]
pub struct TournamentMetadata {
    pub name: String,
    pub tournament_type: TournamentType,
    pub match_settings: TakGameSettings,
}

#[derive(Clone, Debug)]
pub struct Tournament {
    pub metadata: TournamentMetadata,
    pub status: TournamentStatus,
}

#[derive(Clone, Debug)]
pub enum TournamentStatus {
    Upcoming,
    Ongoing,
    Completed,
}

#[derive(Clone, Debug)]
pub enum TournamentType {
    Swiss,
    RoundRobin,
}

#[async_trait::async_trait]
pub trait TournamentRepository {
    async fn create_tournament(&self, tournament: Tournament) -> Result<TournamentId, RepoError>;
    async fn list_tournaments(&self) -> Result<Vec<(TournamentId, Tournament)>, RepoError>;
    async fn get_tournament(
        &self,
        tournament_id: TournamentId,
    ) -> Result<Tournament, RepoRetrieveError>;
    async fn set_tournament_status(
        &self,
        tournament_id: TournamentId,
        status: TournamentStatus,
    ) -> Result<(), RepoError>;
}

#[async_trait::async_trait]
pub trait TournamentPlayerRegistrationRepository {
    async fn get_registered_players(
        &self,
        tournament_id: TournamentId,
    ) -> Result<Vec<PlayerId>, RepoError>;
    async fn register_player(
        &self,
        tournament_id: TournamentId,
        player_id: PlayerId,
    ) -> Result<(), RepoError>;
    async fn unregister_player(
        &self,
        tournament_id: TournamentId,
        player_id: PlayerId,
    ) -> Result<(), RepoError>;
}
