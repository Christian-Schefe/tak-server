use std::sync::Arc;

use dashmap::DashMap;
use tak_core::{TakGameSettings, TakPlayer};

use crate::domain::{MatchId, PlayerId, RepoError, RepoRetrieveError, TournamentId};

#[async_trait::async_trait]
pub trait MatchRepository {
    async fn create_match(&self, new_match: Match) -> Result<MatchId, RepoError>;
    async fn get_match(&self, match_id: MatchId) -> Result<Match, RepoRetrieveError>;
    async fn update_match(&self, match_id: MatchId, updated_match: Match) -> Result<(), RepoError>;
    async fn get_matches_of_tournament(
        &self,
        tournament_id: TournamentId,
    ) -> Result<Vec<(MatchId, Match)>, RepoError>;
}

#[derive(Clone, Debug)]
pub struct Match {
    pub settings: MatchSettings,
    pub player1: PlayerId,
    pub player2: PlayerId,
    pub status: MatchStatus,
    pub games_played: u32,
    pub score_player1: u32,
    pub score_player2: u32,
    pub tournament_info: Option<MatchTournamentInfo>,
    pub initial_color: TakPlayer,
}

#[derive(Clone, Debug)]
pub struct MatchSettings {
    pub game_settings: TakGameSettings,
    pub match_mode: MatchMode,
    pub is_rated: bool,
    // TODO: tiebreak: Option<TiebreakSettings>,
}

#[derive(Clone, Debug)]
pub struct MatchTournamentInfo {
    pub tournament_id: TournamentId,
    pub round: u32,
    pub round_match_number: u32,
}

#[derive(Clone, Debug)]
pub enum MatchMode {
    Unlimited,
    FixedGames(u32),
    FirstTo(u32),
}

impl Match {
    pub fn new(
        player1: PlayerId,
        player2: PlayerId,
        tournament_info: Option<MatchTournamentInfo>,
        settings: MatchSettings,
        initial_color: TakPlayer,
    ) -> Self {
        Self {
            player1,
            player2,
            settings,
            status: MatchStatus::Waiting,
            initial_color,
            games_played: 0,
            score_player1: 0,
            score_player2: 0,
            tournament_info,
        }
    }

    pub fn get_winner(&self) -> Option<PlayerId> {
        match self.status {
            MatchStatus::Completed => {
                if self.score_player1 > self.score_player2 {
                    Some(self.player1)
                } else if self.score_player2 > self.score_player1 {
                    Some(self.player2)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn end_game_in_match(&mut self, winner: Option<PlayerId>) {
        self.games_played += 1;
        match winner {
            Some(winner) => {
                if winner == self.player1 {
                    self.score_player1 += 2;
                } else if winner == self.player2 {
                    self.score_player2 += 2;
                }
            }
            None => {
                self.score_player1 += 1;
                self.score_player2 += 1;
            }
        }
        let completed = match self.settings.match_mode {
            MatchMode::Unlimited => false,
            MatchMode::FixedGames(total_games) => self.games_played >= total_games,
            MatchMode::FirstTo(score) => self.score_player1 >= score || self.score_player2 >= score,
        };
        if completed {
            self.status = MatchStatus::Completed;
        } else {
            self.status = MatchStatus::Waiting;
        }
    }

    pub fn try_begin_game(&mut self) -> Result<TakPlayer, String> {
        match self.status {
            MatchStatus::Waiting => {
                self.status = MatchStatus::InProgress;
                let player1_color = if self.games_played % 2 == 0 {
                    self.initial_color
                } else {
                    self.initial_color.opponent()
                };
                Ok(player1_color)
            }
            MatchStatus::InProgress => Err("Game is already in progress".to_string()),
            MatchStatus::Completed => Err("Match is already completed".to_string()),
        }
    }
}

#[derive(Clone, Debug)]
pub enum MatchStatus {
    Waiting,
    InProgress,
    Completed,
}

pub trait MatchReadinessService {
    fn get_readiness_status(&self, match_id: MatchId) -> Option<PlayerId>;
    fn set_player_ready(&self, match_id: MatchId, player: PlayerId) -> bool;
    fn set_player_not_ready(&self, match_id: MatchId, player: PlayerId) -> bool;
    fn set_player_not_ready_everywhere(&self, player: PlayerId) -> Vec<MatchId>;
}

pub struct MatchReadinessServiceImpl {
    match_readiness: Arc<DashMap<MatchId, PlayerId>>,
}

impl MatchReadinessServiceImpl {
    pub fn new() -> Self {
        Self {
            match_readiness: Arc::new(DashMap::new()),
        }
    }
}

impl MatchReadinessService for MatchReadinessServiceImpl {
    fn get_readiness_status(&self, match_id: MatchId) -> Option<PlayerId> {
        self.match_readiness
            .get(&match_id)
            .map(|entry| *entry.value())
    }

    fn set_player_ready(&self, match_id: MatchId, player: PlayerId) -> bool {
        let request = self.match_readiness.get(&match_id);
        if let Some(already_ready) = &request {
            if *already_ready.value() == player {
                false
            } else {
                drop(request);
                self.match_readiness.remove(&match_id);
                true
            }
        } else {
            drop(request);
            self.match_readiness.insert(match_id, player);
            false
        }
    }

    fn set_player_not_ready(&self, match_id: MatchId, player: PlayerId) -> bool {
        let request = self.match_readiness.get(&match_id);
        if let Some(already_ready) = &request {
            if *already_ready.value() == player {
                drop(request);
                self.match_readiness.remove(&match_id);
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    fn set_player_not_ready_everywhere(&self, player: PlayerId) -> Vec<MatchId> {
        let mut match_ids = Vec::new();
        self.match_readiness.retain(|match_id, ready_player| {
            if *ready_player == player {
                match_ids.push(*match_id);
                false
            } else {
                true
            }
        });
        match_ids
    }
}
