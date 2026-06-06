use std::collections::HashSet;

use tak_core::{TakGameSettings, TakPlayer};

use crate::domain::{
    MatchId, PlayerId, RepoError, RepoRetrieveError, TournamentId, matches::Match,
};

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

pub struct TournamentRound {
    pub matches: Vec<MatchId>,
    pub byes: Vec<PlayerId>,
}

impl TournamentRound {
    pub fn new(matches: Vec<MatchId>, byes: Vec<PlayerId>) -> Self {
        Self { matches, byes }
    }
}

#[derive(Clone, Debug)]
pub struct TournamentPlayer {
    pub player_id: PlayerId,
    pub half_score: u32,
}

impl TournamentPlayer {
    pub fn new(player_id: PlayerId) -> Self {
        Self {
            player_id,
            half_score: 0,
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
        previous_pairings: &[Match],
        round_index: usize,
    ) -> RoundPairing {
        match self {
            TournamentFormat::Swiss { .. } => {
                Self::generate_swiss_pairings(players, previous_pairings, round_index)
            }
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

    fn generate_swiss_pairings(
        players: &[TournamentPlayer],
        previous_pairings: &[Match],
        round_index: usize,
    ) -> RoundPairing {
        fn normalize_pair(a: PlayerId, b: PlayerId) -> (PlayerId, PlayerId) {
            if a.0 < b.0 { (a, b) } else { (b, a) }
        }

        let previous_pairings_set = previous_pairings
            .iter()
            .map(|m| normalize_pair(m.player1, m.player2))
            .collect::<HashSet<_>>();

        let mut sorted = players.to_vec();

        // Highest score first
        sorted.sort_by(|a, b| {
            b.half_score
                .partial_cmp(&a.half_score)
                .unwrap()
                .then_with(|| a.player_id.0.cmp(&b.player_id.0))
        });

        let mut pairings = Vec::new();
        let mut byes = Vec::new();

        let mut used = vec![false; sorted.len()];

        // Bye handling
        if sorted.len() % 2 != 0 {
            // Give bye to the lowest-ranked player
            let bye_player = sorted.last().unwrap().player_id;
            byes.push(bye_player);

            used[sorted.len() - 1] = true;
        }

        for i in 0..sorted.len() {
            if used[i] {
                continue;
            }

            let p1 = sorted[i].player_id;

            // Find next valid opponent
            let mut opponent_index = None;

            for j in (i + 1)..sorted.len() {
                if used[j] {
                    continue;
                }

                let p2 = sorted[j].player_id;

                let pair_key = normalize_pair(p1, p2);

                // Prefer opponents not already played
                if !previous_pairings_set.contains(&pair_key) {
                    opponent_index = Some(j);
                    break;
                }

                // Fallback if no better option exists
                if opponent_index.is_none() {
                    opponent_index = Some(j);
                }
            }

            if let Some(j) = opponent_index {
                used[i] = true;
                used[j] = true;

                let p2 = sorted[j].player_id;

                // Alternate colors by round + board index
                let color = if (pairings.len() + round_index) % 2 == 0 {
                    TakPlayer::White
                } else {
                    TakPlayer::Black
                };

                pairings.push((p1, p2, color));
            }
        }

        RoundPairing { pairings, byes }
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
        player_id: PlayerId,
        score_increase: u32,
    ) -> Result<(), RepoError>;
    async fn remove_tournament_player(
        &self,
        tournament_id: TournamentId,
        player_id: PlayerId,
    ) -> Result<(), RepoError>;
}
