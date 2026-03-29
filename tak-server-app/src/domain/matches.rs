use std::sync::Arc;

use dashmap::DashMap;
use tak_core::{TakGameSettings, TakPlayer};

use crate::domain::{MatchId, PlayerId, RepoError, RepoRetrieveError, TournamentId};

#[async_trait::async_trait]
pub trait MatchRepository {
    async fn create_match(&self, new_match: Match) -> Result<MatchId, RepoError>;
    async fn get_match(&self, match_id: MatchId) -> Result<Match, RepoRetrieveError>;
    async fn get_matches_of_tournament(
        &self,
        tournament_id: TournamentId,
    ) -> Result<Vec<(MatchId, Match)>, RepoError>;
    async fn update_match(&self, match_id: MatchId, updated_match: Match) -> Result<(), RepoError>;
}

#[derive(Clone, Debug)]
pub struct Match {
    pub player1: PlayerId,
    pub player2: PlayerId,
    pub initial_color: TakPlayer,
    pub game_settings: TakGameSettings,
    pub status: MatchStatus,
    pub match_mode: MatchMode,
    pub games_played: u32,
    pub half_score_player1: u32,
    pub half_score_player2: u32,
    pub is_rated: bool,
    pub tournament_info: Option<MatchTournamentInfo>,
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
        initial_color: Option<TakPlayer>,
        game_settings: TakGameSettings,
        match_mode: MatchMode,
        is_rated: bool,
        tournament_id: Option<MatchTournamentInfo>,
    ) -> Self {
        let initial_color = initial_color.unwrap_or_else(|| {
            if rand::random::<bool>() {
                TakPlayer::White
            } else {
                TakPlayer::Black
            }
        });
        Self {
            player1,
            player2,
            initial_color,
            game_settings,
            status: MatchStatus::Waiting,
            match_mode,
            games_played: 0,
            half_score_player1: 0,
            half_score_player2: 0,
            is_rated,
            tournament_info: tournament_id,
        }
    }
    pub fn end_game_in_match(&mut self, winner: Option<PlayerId>) {
        self.games_played += 1;
        match winner {
            Some(winner) => {
                if winner == self.player1 {
                    self.half_score_player1 += 2;
                } else if winner == self.player2 {
                    self.half_score_player2 += 2;
                }
            }
            None => {
                self.half_score_player1 += 1;
                self.half_score_player2 += 1;
            }
        }
        let completed = match self.match_mode {
            MatchMode::Unlimited => false,
            MatchMode::FixedGames(total_games) => self.games_played >= total_games,
            MatchMode::FirstTo(half_score) => {
                self.half_score_player1 >= half_score || self.half_score_player2 >= half_score
            }
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

pub trait RematchService {
    fn request_or_accept_rematch(
        &self,
        match_id: MatchId,
        player: PlayerId,
    ) -> Result<bool, RequestRematchError>;
    fn retract_rematch_request(
        &self,
        match_id: MatchId,
        player: PlayerId,
    ) -> Result<(), RetractRematchError>;
}

#[derive(Debug)]
pub enum RequestRematchError {
    MatchNotFound,
    InvalidPlayer,
    GameInProgress,
}

#[derive(Debug)]
pub enum RetractRematchError {
    MatchNotFound,
    NoRematchRequested,
    GameInProgress,
}

pub struct RematchServiceImpl {
    rematch_requests: Arc<DashMap<MatchId, PlayerId>>,
}

impl RematchServiceImpl {
    pub fn new() -> Self {
        Self {
            rematch_requests: Arc::new(DashMap::new()),
        }
    }
}

impl RematchService for RematchServiceImpl {
    fn request_or_accept_rematch(
        &self,
        match_id: MatchId,
        player: PlayerId,
    ) -> Result<bool, RequestRematchError> {
        let request = self.rematch_requests.get(&match_id);
        if let Some(existing_request) = &request {
            if *existing_request.value() == player {
                Ok(false)
            } else {
                drop(request);
                self.rematch_requests.remove(&match_id);
                Ok(true)
            }
        } else {
            drop(request);
            self.rematch_requests.insert(match_id, player);
            Ok(false)
        }
    }

    fn retract_rematch_request(
        &self,
        match_id: MatchId,
        player: PlayerId,
    ) -> Result<(), RetractRematchError> {
        let request = self.rematch_requests.get(&match_id);
        if let Some(existing_request) = &request {
            if *existing_request.value() == player {
                drop(request);
                self.rematch_requests.remove(&match_id);
                Ok(())
            } else {
                Err(RetractRematchError::NoRematchRequested)
            }
        } else {
            Err(RetractRematchError::NoRematchRequested)
        }
    }
}
