use std::sync::Arc;

use dashmap::DashMap;
use tak_core::{TakGameSettings, TakPlayer};

use crate::domain::{MatchId, UserId, RepoError, RepoRetrieveError, TournamentId};

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
    pub player1: MatchPlayer,
    pub player2: MatchPlayer,
    pub status: MatchStatus,
    pub games_played: u32,
    pub tournament_info: Option<MatchTournamentInfo>,
    pub initial_color: TakPlayer,
}

#[derive(Clone, Debug)]
pub struct MatchPlayer {
    pub user_id: UserId,
    pub score: u32,
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
        player1: UserId,
        player2: UserId,
        tournament_info: Option<MatchTournamentInfo>,
        settings: MatchSettings,
        initial_color: TakPlayer,
    ) -> Self {
        Self {
            player1: MatchPlayer {
                user_id: player1,
                score: 0,
            },
            player2: MatchPlayer {
                user_id: player2,
                score: 0,
            },
            settings,
            status: MatchStatus::Waiting,
            initial_color,
            games_played: 0,
            tournament_info,
        }
    }

    pub fn get_winner(&self) -> Option<UserId> {
        match self.status {
            MatchStatus::Completed => {
                if self.player1.score > self.player2.score {
                    Some(self.player1.user_id)
                } else if self.player2.score > self.player1.score {
                    Some(self.player2.user_id)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn abort_game_in_match(&mut self) {
        self.status = MatchStatus::Waiting;
    }

    pub fn end_game_in_match(&mut self, winner: Option<UserId>) {
        match winner {
            Some(winner) => {
                if winner == self.player1.user_id {
                    self.player1.score += 2;
                } else if winner == self.player2.user_id {
                    self.player2.score += 2;
                } else {
                    tracing::error!(
                        "Winner {} is not part of the match between {} and {}",
                        winner,
                        self.player1.user_id,
                        self.player2.user_id
                    );
                    return;
                }
            }
            None => {
                self.player1.score += 1;
                self.player2.score += 1;
            }
        }
        self.games_played += 1;
        let completed = match self.settings.match_mode {
            MatchMode::Unlimited => false,
            MatchMode::FixedGames(total_games) => self.games_played >= total_games,
            MatchMode::FirstTo(score) => self.player1.score >= score || self.player2.score >= score,
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
                self.status = MatchStatus::Ongoing;
                let player1_color = if self.games_played % 2 == 0 {
                    self.initial_color
                } else {
                    self.initial_color.opponent()
                };
                Ok(player1_color)
            }
            MatchStatus::Ongoing => Err("Game is already in progress".to_string()),
            MatchStatus::Completed => Err("Match is already completed".to_string()),
        }
    }
}

#[derive(Clone, Debug)]
pub enum MatchStatus {
    Waiting,
    Ongoing,
    Completed,
}

pub trait MatchReadinessService {
    fn get_readiness_status(&self, match_id: MatchId) -> Option<UserId>;
    fn set_player_ready(&self, match_id: MatchId, player: UserId) -> bool;
    fn set_player_not_ready(&self, match_id: MatchId, player: UserId) -> bool;
    fn set_player_not_ready_everywhere(&self, player: UserId) -> Vec<MatchId>;
}

pub struct MatchReadinessServiceImpl {
    match_readiness: Arc<DashMap<MatchId, UserId>>,
}

impl MatchReadinessServiceImpl {
    pub fn new() -> Self {
        Self {
            match_readiness: Arc::new(DashMap::new()),
        }
    }
}

impl MatchReadinessService for MatchReadinessServiceImpl {
    fn get_readiness_status(&self, match_id: MatchId) -> Option<UserId> {
        self.match_readiness
            .get(&match_id)
            .map(|entry| *entry.value())
    }

    fn set_player_ready(&self, match_id: MatchId, player: UserId) -> bool {
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

    fn set_player_not_ready(&self, match_id: MatchId, player: UserId) -> bool {
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

    fn set_player_not_ready_everywhere(&self, player: UserId) -> Vec<MatchId> {
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
