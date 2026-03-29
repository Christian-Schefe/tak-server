use tak_core::{TakGameSettings, TakPlayer};

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

impl TournamentType {
    pub fn generate_pairings(
        &self,
        players: &[PlayerId],
        round_index: usize,
    ) -> Vec<(PlayerId, PlayerId, TakPlayer)> {
        match self {
            TournamentType::Swiss => todo!(),
            TournamentType::RoundRobin => Self::generate_round_robin_pairings(players, round_index),
        }
    }

    fn generate_round_robin_pairings(
        players: &[PlayerId],
        round_index: usize,
    ) -> Vec<(PlayerId, PlayerId, TakPlayer)> {
        let mut players: Vec<Option<PlayerId>> = players.iter().map(|x| Some(*x)).collect();
        if players.len() % 2 != 0 {
            players.push(None);
        }
        let n = players.len();
        let color = if round_index % 2 == 0 {
            TakPlayer::White
        } else {
            TakPlayer::Black
        };

        // Rotate players except the first one
        let rotation = round_index % (n - 1);
        players[1..].rotate_right(rotation);

        let mut pairings = Vec::new();

        for i in 0..(n / 2) {
            let p1 = players[i];
            let p2 = players[n - 1 - i];
            if let (Some(p1), Some(p2)) = (p1, p2) {
                pairings.push((p1, p2, color));
            }
        }

        pairings
    }
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
