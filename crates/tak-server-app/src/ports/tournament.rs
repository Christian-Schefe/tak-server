use tak_domain::{
    TournamentId, UserId,
    tournament::{Tournament, TournamentPlayer, TournamentRound, TournamentStatus},
};

use crate::ports::{RepoError, RepoRetrieveError};

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
pub trait TournamentRoundRepository {
    async fn get_tournament_rounds(
        &self,
        tournament_id: TournamentId,
    ) -> Result<Vec<TournamentRound>, RepoError>;
    async fn create_tournament_round(
        &self,
        tournament_id: TournamentId,
        round_index: usize,
        tournament_round: TournamentRound,
    ) -> Result<(), RepoError>;
}

#[async_trait::async_trait]
pub trait TournamentPlayerRepository {
    async fn get_tournament_players(
        &self,
        tournament_id: TournamentId,
    ) -> Result<Vec<TournamentPlayer>, RepoError>;
    async fn create_tournament_player(
        &self,
        tournament_id: TournamentId,
        player: TournamentPlayer,
    ) -> Result<(), RepoError>;
    async fn increase_player_score(
        &self,
        tournament_id: TournamentId,
        user_id: UserId,
        score_increase: u32,
    ) -> Result<(), RepoError>;
    async fn remove_tournament_player(
        &self,
        tournament_id: TournamentId,
        user_id: UserId,
    ) -> Result<(), RepoError>;
    async fn set_player_seeding_score(
        &self,
        tournament_id: TournamentId,
        user_id: UserId,
        seeding_score: i32,
    ) -> Result<(), RepoError>;
}
