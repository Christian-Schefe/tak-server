use tak_core::{TakGameSettings, TakPlayer};

use crate::domain::{PlayerId, RepoError, RepoRetrieveError, TournamentId};

#[derive(Clone, Debug)]
pub struct TournamentMetadata {
    pub name: String,
    pub tournament_format: TournamentFormat,
    pub match_settings: TakGameSettings,
}

#[derive(Clone, Debug)]
pub struct Tournament {
    pub metadata: TournamentMetadata,
    pub status: TournamentStatus,
}

#[derive(Clone, Debug)]
pub struct TournamentPlayer {
    pub player_id: PlayerId,
    pub score: u32,
}

impl TournamentPlayer {
    pub fn new(player_id: PlayerId) -> Self {
        Self {
            player_id,
            score: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub enum TournamentStatus {
    Upcoming,
    Ongoing,
    Completed,
}

#[derive(Clone, Debug)]
pub enum TournamentFormat {
    Swiss { rounds: usize },
    RoundRobin,
}

pub struct RoundPairing {
    pub pairings: Vec<(PlayerId, PlayerId, TakPlayer)>,
    pub byes: Vec<PlayerId>,
}

impl TournamentFormat {
    pub fn generate_pairings(
        &self,
        players: &[TournamentPlayer],
        round_index: usize,
    ) -> RoundPairing {
        match self {
            TournamentFormat::Swiss { .. } => todo!(),
            TournamentFormat::RoundRobin => {
                Self::generate_round_robin_pairings(players, round_index)
            }
        }
    }

    pub fn is_finished(&self, players: &[TournamentPlayer], round_index: usize) -> bool {
        match self {
            TournamentFormat::Swiss { rounds } => round_index >= *rounds,
            TournamentFormat::RoundRobin => {
                let total_rounds = if players.len() % 2 == 0 {
                    players.len() - 1
                } else {
                    players.len()
                };
                round_index >= total_rounds
            }
        }
    }

    fn generate_round_robin_pairings(
        players: &[TournamentPlayer],
        round_index: usize,
    ) -> RoundPairing {
        let mut players: Vec<Option<PlayerId>> =
            players.iter().map(|tp| Some(tp.player_id)).collect();
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

        let mut byes = Vec::new();

        for i in 0..(n / 2) {
            let p1 = players[i];
            let p2 = players[n - 1 - i];
            match (p1, p2) {
                (Some(p1), Some(p2)) => pairings.push((p1, p2, color)),
                (Some(p1), None) => byes.push(p1),
                (None, Some(p2)) => byes.push(p2),
                (None, None) => {}
            }
        }

        RoundPairing { pairings, byes }
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
        player_id: PlayerId,
        score_increase: u32,
    ) -> Result<(), RepoError>;
    async fn remove_tournament_player(
        &self,
        tournament_id: TournamentId,
        player_id: PlayerId,
    ) -> Result<(), RepoError>;
}
