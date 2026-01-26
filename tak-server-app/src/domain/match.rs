use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use dashmap::DashMap;
use tak_core::{TakGameSettings, TakPlayer};

use crate::domain::{GameId, MatchId, PlayerId};

#[derive(Clone, Debug)]
pub struct Match {
    pub id: MatchId,
    pub player1: PlayerId,
    pub player2: PlayerId,
    pub initial_color: TakPlayer,
    pub color_rule: MatchColorRule,
    pub game_settings: TakGameSettings,
    pub is_rated: bool,
    pub played_games: Vec<GameId>,
    pub status: MatchStatus,
    rematch_requested_by: Option<PlayerId>,
    last_game_finished: Option<Instant>,
}

#[derive(Clone, Debug)]
pub enum MatchStatus {
    Waiting,
    InProgressReserved,
    InProgress(GameId),
}

impl Match {
    pub fn get_next_matchup_colors(&self) -> (PlayerId, PlayerId) {
        let color = match self.color_rule {
            MatchColorRule::Keep => Some(self.initial_color),
            MatchColorRule::Alternate => {
                if self.played_games.len() % 2 == 0 {
                    Some(self.initial_color)
                } else {
                    match self.initial_color {
                        TakPlayer::White => Some(TakPlayer::Black),
                        TakPlayer::Black => Some(TakPlayer::White),
                    }
                }
            }
            MatchColorRule::Random => None,
        };
        match color {
            Some(TakPlayer::White) => (self.player1, self.player2),
            Some(TakPlayer::Black) => (self.player2, self.player1),
            None => {
                if rand::random() {
                    (self.player1, self.player2)
                } else {
                    (self.player2, self.player1)
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum MatchColorRule {
    Keep,
    Alternate,
    Random,
}

pub trait MatchService {
    fn create_match(
        &self,
        player1: PlayerId,
        player2: PlayerId,
        initial_color: Option<TakPlayer>,
        color_rule: MatchColorRule,
        game_settings: TakGameSettings,
        is_rated: bool,
    ) -> MatchId;
    fn get_match(&self, match_id: MatchId) -> Option<Match>;
    fn reserve_match_in_progress(&self, match_id: MatchId) -> Option<Match>;
    fn start_game_in_match(&self, match_id: MatchId, game_id: GameId) -> bool;
    fn end_game_in_match(&self, match_id: MatchId, game_id: GameId) -> bool;
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
    fn cleanup_old_matches(&self, now: Instant);
    fn get_match_id_by_game_id(&self, game_id: GameId) -> Option<MatchId>;
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

pub struct MatchServiceImpl {
    next_match_id: Arc<Mutex<MatchId>>,
    matches: Arc<DashMap<MatchId, Match>>,
    matches_by_game: Arc<DashMap<GameId, MatchId>>,
}

impl MatchServiceImpl {
    pub fn new() -> Self {
        Self {
            next_match_id: Arc::new(Mutex::new(MatchId(0))),
            matches: Arc::new(DashMap::new()),
            matches_by_game: Arc::new(DashMap::new()),
        }
    }

    fn generate_match_id(&self) -> MatchId {
        let mut lock = self.next_match_id.lock().unwrap();
        let match_id = *lock;
        lock.0 += 1;
        match_id
    }
}

impl MatchService for MatchServiceImpl {
    fn create_match(
        &self,
        player1: PlayerId,
        player2: PlayerId,
        initial_color: Option<TakPlayer>,
        color_rule: MatchColorRule,
        game_settings: TakGameSettings,
        is_rated: bool,
    ) -> MatchId {
        let match_id = self.generate_match_id();
        let initial_color = match initial_color {
            Some(color) => color,
            None => {
                if rand::random() {
                    TakPlayer::White
                } else {
                    TakPlayer::Black
                }
            }
        };
        let new_match = Match {
            id: match_id,
            player1,
            player2,
            initial_color,
            color_rule,
            game_settings,
            is_rated,
            played_games: Vec::new(),
            status: MatchStatus::Waiting,
            rematch_requested_by: None,
            last_game_finished: None,
        };
        self.matches.insert(match_id, new_match.clone());
        match_id
    }

    fn reserve_match_in_progress(&self, match_id: MatchId) -> Option<Match> {
        if let Some(mut match_entry) = self.matches.get_mut(&match_id) {
            let MatchStatus::Waiting = match_entry.status else {
                return None;
            };
            match_entry.status = MatchStatus::InProgressReserved;
            Some(match_entry.clone())
        } else {
            None
        }
    }

    fn start_game_in_match(&self, match_id: MatchId, game_id: GameId) -> bool {
        if let Some(mut match_entry) = self.matches.get_mut(&match_id) {
            let MatchStatus::InProgressReserved = match_entry.status else {
                return false;
            };
            match_entry.status = MatchStatus::InProgress(game_id);
            match_entry.rematch_requested_by = None;
            self.matches_by_game.insert(game_id, match_id);
            true
        } else {
            false
        }
    }

    fn end_game_in_match(&self, match_id: MatchId, game_id: GameId) -> bool {
        if let Some(mut match_entry) = self.matches.get_mut(&match_id) {
            let MatchStatus::InProgress(current_game_id) = match_entry.status else {
                return false;
            };
            if game_id != current_game_id {
                return false;
            }
            match_entry.played_games.push(current_game_id);
            match_entry.status = MatchStatus::Waiting;
            match_entry.last_game_finished = Some(Instant::now());
            true
        } else {
            false
        }
    }

    fn request_or_accept_rematch(
        &self,
        match_id: MatchId,
        player: PlayerId,
    ) -> Result<bool, RequestRematchError> {
        if let Some(mut match_entry) = self.matches.get_mut(&match_id) {
            if match_entry.player1 != player && match_entry.player2 != player {
                return Err(RequestRematchError::InvalidPlayer);
            }
            let MatchStatus::Waiting = match_entry.status else {
                return Err(RequestRematchError::GameInProgress);
            };
            if let Some(requester) = match_entry.rematch_requested_by {
                if requester != player {
                    match_entry.rematch_requested_by = None;
                    return Ok(true);
                } else {
                    return Ok(false);
                }
            } else {
                match_entry.rematch_requested_by = Some(player);
                return Ok(false);
            }
        }
        Err(RequestRematchError::MatchNotFound)
    }

    fn retract_rematch_request(
        &self,
        match_id: MatchId,
        player: PlayerId,
    ) -> Result<(), RetractRematchError> {
        if let Some(mut match_entry) = self.matches.get_mut(&match_id) {
            let MatchStatus::Waiting = match_entry.status else {
                return Err(RetractRematchError::GameInProgress);
            };
            if match_entry.rematch_requested_by == Some(player) {
                match_entry.rematch_requested_by = None;
                Ok(())
            } else {
                Err(RetractRematchError::NoRematchRequested)
            }
        } else {
            Err(RetractRematchError::MatchNotFound)
        }
    }

    fn get_match(&self, match_id: MatchId) -> Option<Match> {
        self.matches.get(&match_id).map(|entry| entry.clone())
    }

    fn cleanup_old_matches(&self, now: Instant) {
        let keys_to_remove: Vec<MatchId> = self
            .matches
            .iter()
            .filter_map(|entry| {
                let match_entry = entry.value();
                if let MatchStatus::Waiting = match_entry.status
                    && let Some(last_finished) = match_entry.last_game_finished
                {
                    let elapsed = now.saturating_duration_since(last_finished);
                    if elapsed > Duration::from_secs(60 * 60 * 5) {
                        return Some(match_entry.id);
                    }
                }
                None
            })
            .collect();

        for key in keys_to_remove {
            log::info!("Cleaning up old match {}", key);
            if let Some((_, match_entry)) = self.matches.remove(&key) {
                for game_id in match_entry.played_games {
                    self.matches_by_game.remove(&game_id);
                }
            }
        }
    }

    fn get_match_id_by_game_id(&self, game_id: GameId) -> Option<MatchId> {
        self.matches_by_game
            .get(&game_id)
            .map(|entry| *entry.value())
    }
}
