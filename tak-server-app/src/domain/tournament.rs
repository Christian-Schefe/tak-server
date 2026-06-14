use std::collections::HashSet;

use tak_core::TakPlayer;

use crate::domain::{
    MatchId, PlayerId, RepoError, RepoRetrieveError, TournamentId,
    matches::{Match, MatchSettings},
};

#[derive(Clone, Debug)]
pub struct TournamentMetadata {
    pub name: String,
    pub tournament_format: TournamentFormat,
    pub match_settings: MatchSettings,
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
    pub score: u32,
    pub seeding_score: i32,
}

impl TournamentPlayer {
    pub fn new(player_id: PlayerId) -> Self {
        Self {
            player_id,
            score: 0,
            seeding_score: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub enum TournamentStatus {
    Upcoming { registration_open: bool },
    Ongoing,
    Completed,
}

#[derive(Clone, Debug)]
pub enum TournamentFormat {
    Swiss { rounds: usize },
    RoundRobin,
    GroupRoundRobin { group_size: usize },
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
    ) -> Option<RoundPairing> {
        match self {
            TournamentFormat::Swiss { rounds } => {
                Self::generate_swiss_pairings(players, previous_pairings, round_index, *rounds)
            }
            TournamentFormat::RoundRobin => {
                Self::generate_round_robin_pairings(players, round_index)
            }
            TournamentFormat::GroupRoundRobin { group_size } => {
                Self::generate_group_round_robin_pairings(players, *group_size, round_index)
            }
        }
    }

    fn generate_swiss_pairings(
        players: &[TournamentPlayer],
        previous_pairings: &[Match],
        round_index: usize,
        rounds: usize,
    ) -> Option<RoundPairing> {
        if round_index >= rounds {
            return None;
        }
        fn normalize_pair(a: PlayerId, b: PlayerId) -> (PlayerId, PlayerId) {
            if a.0 < b.0 { (a, b) } else { (b, a) }
        }

        let previous_pairings_set = previous_pairings
            .iter()
            .map(|m| normalize_pair(m.player1.player_id, m.player2.player_id))
            .collect::<HashSet<_>>();

        let mut sorted = players.iter().enumerate().collect::<Vec<_>>();

        // Highest score first, use seeding order as tiebreaker
        sorted.sort_by_key(|(index, player)| (std::cmp::Reverse(player.score), *index));

        let mut pairings = Vec::new();
        let mut byes = Vec::new();

        let mut used = vec![false; sorted.len()];

        // Bye handling
        if sorted.len() % 2 != 0 {
            // Give bye to the lowest-ranked player
            let bye_player = sorted.last().unwrap().1.player_id;
            byes.push(bye_player);

            used[sorted.len() - 1] = true;
        }

        for i in 0..sorted.len() {
            if used[i] {
                continue;
            }

            let p1 = sorted[i].1.player_id;

            // Find next valid opponent
            let mut opponent_index = None;

            for j in (i + 1)..sorted.len() {
                if used[j] {
                    continue;
                }

                let p2 = sorted[j].1.player_id;

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

                let p2 = sorted[j].1.player_id;

                // Alternate colors by round + board index
                let color = if (pairings.len() + round_index) % 2 == 0 {
                    TakPlayer::White
                } else {
                    TakPlayer::Black
                };

                pairings.push((p1, p2, color));
            }
        }

        Some(RoundPairing { pairings, byes })
    }

    fn generate_round_robin_pairings(
        players: &[TournamentPlayer],
        round_index: usize,
    ) -> Option<RoundPairing> {
        let total_rounds = if players.len() % 2 == 0 {
            players.len() - 1
        } else {
            players.len()
        };
        if round_index >= total_rounds {
            return None;
        }
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

        Some(RoundPairing { pairings, byes })
    }

    fn generate_group_round_robin_pairings(
        players: &[TournamentPlayer],
        group_size: usize,
        round_index: usize,
    ) -> Option<RoundPairing> {
        let mut pairings = Vec::new();
        let mut byes = Vec::new();
        let mut has_unfinished_group = false;

        let group_count = if group_size > 0 {
            (players.len() + group_size - 1) / group_size
        } else {
            0
        };
        let mut groups = vec![Vec::new(); group_count];
        for (i, player) in players.iter().enumerate() {
            groups[i % group_count].push(player.clone());
        }

        for group in groups {
            let round_pairing = Self::generate_round_robin_pairings(&group, round_index);
            if let Some(rp) = round_pairing {
                pairings.extend(rp.pairings);
                byes.extend(rp.byes);
                has_unfinished_group = true;
            }
        }

        if !has_unfinished_group {
            return None;
        }

        Some(RoundPairing { pairings, byes })
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
    async fn set_player_seeding_score(
        &self,
        tournament_id: TournamentId,
        player_id: PlayerId,
        seeding_score: i32,
    ) -> Result<(), RepoError>;
}
