use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use crate::tak::{
    TakAction, TakGameSettings, TakGameState, TakPlayer, TakVariant, TakWinReason, board::TakBoard,
};

#[derive(Clone, Debug)]
pub struct TakBaseGame {
    pub settings: TakGameSettings,
    pub board: TakBoard,
    pub current_player: TakPlayer,
    pub reserves: (TakReserve, TakReserve),
    pub game_state: TakGameState,
    pub board_hash_history: HashMap<String, u32>,
    pub ply_index: usize,
}

impl TakBaseGame {
    pub fn new(settings: TakGameSettings) -> Self {
        let board = TakBoard::new(settings.board_size);
        let reserve = TakReserve {
            pieces: settings.reserve_pieces,
            capstones: settings.reserve_capstones,
        };
        let reserves = (reserve.clone(), reserve);
        TakBaseGame {
            settings,
            board,
            current_player: TakPlayer::White,
            reserves,
            game_state: TakGameState::Ongoing,
            board_hash_history: HashMap::new(),
            ply_index: 0,
        }
    }

    pub fn can_do_action(&self, action: &TakAction) -> Result<(), String> {
        if self.game_state != TakGameState::Ongoing {
            return Err("Game is already over".to_string());
        }

        match action {
            TakAction::Place { pos, variant } => {
                self.board.can_do_place(pos)?;
                if self.ply_index < 2 && *variant != TakVariant::Flat {
                    return Err("First two moves must be flat stones".to_string());
                }
                let reserve = match self.current_player {
                    TakPlayer::White => &self.reserves.0,
                    TakPlayer::Black => &self.reserves.1,
                };
                let amount = match variant {
                    TakVariant::Flat | TakVariant::Standing => reserve.pieces,
                    TakVariant::Capstone => reserve.capstones,
                };
                if amount == 0 {
                    return Err("No pieces of the specified variant left in reserve".to_string());
                }
                Ok(())
            }
            TakAction::Move { pos, dir, drops } => {
                self.board.can_do_move(pos, dir, drops)?;
                if self.ply_index < 2 {
                    return Err("Cannot move pieces before both players have placed at least one piece each".to_string());
                }
                Ok(())
            }
        }
    }

    pub fn do_action(&mut self, action: &TakAction) -> Result<(), String> {
        self.can_do_action(&action)?;
        match &action {
            TakAction::Place { pos, variant } => {
                let placing_player = if self.ply_index < 2 {
                    self.current_player.opponent()
                } else {
                    self.current_player.clone()
                };
                self.board.do_place(pos, variant, &placing_player)?;
                let reserve = match self.current_player {
                    TakPlayer::White => &mut self.reserves.0,
                    TakPlayer::Black => &mut self.reserves.1,
                };
                let amount = match variant {
                    TakVariant::Flat | TakVariant::Standing => &mut reserve.pieces,
                    TakVariant::Capstone => &mut reserve.capstones,
                };
                *amount -= 1;
            }
            TakAction::Move { pos, dir, drops } => {
                self.board.do_move(pos, dir, drops)?;
            }
        }

        self.check_game_over();

        self.current_player = self.current_player.opponent();
        self.ply_index += 1;

        Ok(())
    }

    fn check_game_over(&mut self) {
        let white_reserve_empty = self.reserves.0.pieces == 0 && self.reserves.0.capstones == 0;
        let black_reserve_empty = self.reserves.1.pieces == 0 && self.reserves.1.capstones == 0;

        if self.board.check_for_road(&self.current_player) {
            self.game_state = TakGameState::Win {
                winner: self.current_player.clone(),
                reason: TakWinReason::Road,
            };
        } else if self.board.check_for_road(&self.current_player.opponent()) {
            self.game_state = TakGameState::Win {
                winner: self.current_player.opponent(),
                reason: TakWinReason::Road,
            };
        } else if self.board.is_full() || white_reserve_empty || black_reserve_empty {
            let (white_flats, black_flats) = self.board.count_flats();
            let (white_score, black_score) =
                (white_flats * 2, black_flats * 2 + self.settings.half_komi);
            self.game_state = match white_score.cmp(&black_score) {
                std::cmp::Ordering::Greater => TakGameState::Win {
                    winner: TakPlayer::White,
                    reason: TakWinReason::Flats,
                },
                std::cmp::Ordering::Less => TakGameState::Win {
                    winner: TakPlayer::Black,
                    reason: TakWinReason::Flats,
                },
                std::cmp::Ordering::Equal => TakGameState::Draw,
            };
        }

        let repeat_count = self
            .board_hash_history
            .entry(self.board.compute_hash_string())
            .and_modify(|e| *e += 1)
            .or_insert(1);
        if *repeat_count >= 3 {
            self.game_state = TakGameState::Draw;
        }
    }
}

#[derive(Clone, Debug)]
pub struct TakGame {
    pub base: TakBaseGame,
    pub action_history: Vec<TakAction>,
    pub draw_offered: (bool, bool),
    pub undo_requested: (bool, bool),
    pub clock: TakClock,
}

#[derive(Clone, Debug)]
pub struct TakReserve {
    pub pieces: u32,
    pub capstones: u32,
}

#[derive(Clone, Debug)]
pub struct TakClock {
    pub remaining_time: (Duration, Duration),
    pub last_update_timestamp: Option<Instant>,
    pub has_gained_extra_time: (bool, bool),
}

impl TakGame {
    pub fn new(settings: TakGameSettings) -> Self {
        let base_game = TakBaseGame::new(settings.clone());
        TakGame {
            base: base_game,
            action_history: Vec::new(),
            draw_offered: (false, false),
            undo_requested: (false, false),
            clock: TakClock {
                remaining_time: (
                    settings.time_control.contingent,
                    settings.time_control.contingent,
                ),
                last_update_timestamp: None,
                has_gained_extra_time: (false, false),
            },
        }
    }

    pub fn is_ongoing(&self) -> bool {
        self.base.game_state == TakGameState::Ongoing
    }

    pub fn check_timeout(&mut self, now: Instant) -> bool {
        if self.base.game_state != TakGameState::Ongoing {
            return false;
        }
        let time_remaining = self.get_time_remaining(&self.base.current_player, now);
        if time_remaining.is_zero() {
            self.base.game_state = TakGameState::Win {
                winner: self.base.current_player.opponent(),
                reason: TakWinReason::Default,
            };
            true
        } else {
            false
        }
    }

    pub fn get_time_remaining(&self, player: &TakPlayer, now: Instant) -> Duration {
        match player {
            TakPlayer::White => self.clock.remaining_time.0,
            TakPlayer::Black => self.clock.remaining_time.1,
        }
        .saturating_sub(
            self.clock
                .last_update_timestamp
                .filter(|_| &self.base.current_player == player)
                .map_or(Duration::ZERO, |last_update| {
                    now.saturating_duration_since(last_update)
                }),
        )
    }

    pub fn get_time_remaining_both(&self, now: Instant) -> (Duration, Duration) {
        (
            self.get_time_remaining(&TakPlayer::White, now),
            self.get_time_remaining(&TakPlayer::Black, now),
        )
    }

    pub fn do_action(&mut self, action: &TakAction) -> Result<(), String> {
        let now = Instant::now();
        let player = self.base.current_player.clone();

        if self.check_timeout(now) {
            return Err("Game is already over due to timeout".to_string());
        }

        self.base.do_action(action)?;
        self.update_clock(now, &player);

        self.action_history.push(action.clone());

        Ok(())
    }

    fn update_clock(&mut self, now: Instant, player: &TakPlayer) {
        let remaining = match player {
            TakPlayer::White => &mut self.clock.remaining_time.0,
            TakPlayer::Black => &mut self.clock.remaining_time.1,
        };
        let time_control = &self.base.settings.time_control;
        if let Some(last_update) = self.clock.last_update_timestamp {
            let elapsed = now.duration_since(last_update);
            *remaining = remaining
                .saturating_sub(elapsed)
                .saturating_add(time_control.increment);
            if let Some((extra_move_index, extra_time)) = time_control.extra {
                let has_gained_extra_time = match player {
                    TakPlayer::White => &mut self.clock.has_gained_extra_time.0,
                    TakPlayer::Black => &mut self.clock.has_gained_extra_time.1,
                };
                let move_index = (self.base.ply_index / 2) + 1;
                if !*has_gained_extra_time && extra_move_index as usize == move_index {
                    *remaining = remaining.saturating_add(extra_time);
                    *has_gained_extra_time = true;
                }
            }
        }
        self.clock.last_update_timestamp = Some(now);
    }

    pub fn resign(&mut self, player: &TakPlayer) -> Result<(), String> {
        let now = Instant::now();
        if self.check_timeout(now) {
            return Err("Game is already over due to timeout".to_string());
        }
        if self.base.game_state != TakGameState::Ongoing {
            return Err("Game is already over".to_string());
        }
        self.base.game_state = TakGameState::Win {
            winner: player.opponent(),
            reason: TakWinReason::Default,
        };
        Ok(())
    }

    pub fn offer_draw(&mut self, player: &TakPlayer, offer: bool) -> Result<bool, String> {
        if self.base.game_state != TakGameState::Ongoing {
            return Err("Game is already over".to_string());
        }
        match player {
            TakPlayer::White => self.draw_offered.0 = offer,
            TakPlayer::Black => self.draw_offered.1 = offer,
        }
        if self.draw_offered.0 && self.draw_offered.1 {
            self.base.game_state = TakGameState::Draw;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn request_undo(&mut self, player: &TakPlayer, request: bool) -> Result<bool, String> {
        if self.base.game_state != TakGameState::Ongoing {
            return Err("Game is already over".to_string());
        }
        match player {
            TakPlayer::White => self.undo_requested.0 = request,
            TakPlayer::Black => self.undo_requested.1 = request,
        }
        if self.undo_requested.0 && self.undo_requested.1 {
            self.undo_action().ok();
            self.undo_requested = (false, false);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn undo_action(&mut self) -> Result<(), String> {
        let now = Instant::now();
        if self.base.game_state != TakGameState::Ongoing {
            return Err("Cannot undo action in a finished game".to_string());
        }
        if self.action_history.pop().is_none() {
            return Err("No actions to undo".to_string());
        };
        let player = self.base.current_player.clone();
        let mut game_clone = TakBaseGame::new(self.base.settings.clone());
        for action in &self.action_history {
            game_clone.do_action(action)?;
        }
        self.base = game_clone;
        self.update_clock(now, &player);
        Ok(())
    }
}
