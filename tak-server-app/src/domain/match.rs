use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use dashmap::DashMap;
use tak_core::{TakGameSettings, TakPlayer};

use crate::domain::{GameId, GameType, MatchId, PlayerId};

#[derive(Clone, Debug)]
pub struct Match {
    pub id: MatchId,
    pub player1: PlayerId,
    pub player2: PlayerId,
    pub initial_color: TakPlayer,
    pub color_rule: MatchColorRule,
    pub game_settings: TakGameSettings,
    pub game_type: GameType,
    pub played_games: Vec<GameId>,
    pub current_game: Option<GameId>,
    rematch_requested_by: Option<PlayerId>,
    last_game_finished: Option<Instant>,
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
        game_type: GameType,
    ) -> Match;
    fn get_match(&self, match_id: MatchId) -> Option<Match>;
    fn start_game_in_match(&self, match_id: MatchId, game_id: GameId) -> bool;
    fn end_game_in_match(&self, match_id: MatchId) -> bool;
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
}

pub enum RequestRematchError {
    MatchNotFound,
    InvalidPlayer,
}

pub enum RetractRematchError {
    MatchNotFound,
    NoRematchRequested,
}

pub struct MatchServiceImpl {
    next_match_id: Arc<Mutex<MatchId>>,
    matches: Arc<DashMap<MatchId, Match>>,
}

impl MatchServiceImpl {
    pub fn new() -> Self {
        Self {
            next_match_id: Arc::new(Mutex::new(MatchId(0))),
            matches: Arc::new(DashMap::new()),
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
        game_type: GameType,
    ) -> Match {
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
            game_type,
            played_games: Vec::new(),
            current_game: None,
            rematch_requested_by: None,
            last_game_finished: None,
        };
        self.matches.insert(match_id, new_match.clone());
        new_match
    }

    fn start_game_in_match(&self, match_id: MatchId, game_id: GameId) -> bool {
        if let Some(mut match_entry) = self.matches.get_mut(&match_id) {
            if match_entry.current_game.is_some() {
                return false;
            }
            match_entry.current_game = Some(game_id);
            true
        } else {
            false
        }
    }

    fn end_game_in_match(&self, match_id: MatchId) -> bool {
        if let Some(mut match_entry) = self.matches.get_mut(&match_id) {
            let Some(current_game_id) = match_entry.current_game else {
                return false;
            };
            match_entry.played_games.push(current_game_id);
            match_entry.current_game = None;
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
                if let Some(last_finished) = match_entry.last_game_finished {
                    let elapsed = now.saturating_duration_since(last_finished);
                    if elapsed > Duration::from_secs(60 * 60 * 5)
                        && match_entry.current_game.is_none()
                    {
                        return Some(match_entry.id);
                    }
                }
                None
            })
            .collect();

        for key in keys_to_remove {
            self.matches.remove(&key);
        }
    }
}
