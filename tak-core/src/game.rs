use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use crate::{
    InvalidActionReason, InvalidPlaceReason, MaybeTimeout, TakAction, TakGameResult,
    TakGameSettings, TakPlayer, TakReserve, TakVariant, TakWinReason, board::TakBoard,
};

#[derive(Clone, Debug)]
pub struct TakFinishedBaseGame {
    game_result: TakGameResult,
    action_history: Vec<TakAction>,
}

impl TakFinishedBaseGame {
    fn new(ended_game: TakOngoingBaseGame, game_result: TakGameResult) -> Self {
        TakFinishedBaseGame {
            game_result,
            action_history: ended_game.action_history,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TakOngoingBaseGame {
    settings: TakGameSettings,
    board: TakBoard,
    current_player: TakPlayer,
    reserves: (TakReserve, TakReserve),
    board_hash_history: HashMap<String, u32>,
    action_history: Vec<TakAction>,
}

#[derive(Clone, Debug)]
pub enum TakBaseGame {
    Ongoing(TakOngoingBaseGame),
    Finished(TakFinishedBaseGame),
}

impl TakOngoingBaseGame {
    fn new(settings: TakGameSettings) -> Self {
        let board = TakBoard::new(settings.board_size);
        let reserves = (settings.reserve.clone(), settings.reserve.clone());
        TakOngoingBaseGame {
            settings,
            board,
            current_player: TakPlayer::White,
            reserves,
            board_hash_history: HashMap::new(),
            action_history: Vec::new(),
        }
    }

    fn can_do_action(&self, action: &TakAction) -> Result<(), InvalidActionReason> {
        match action {
            TakAction::Place { pos, variant } => {
                if self.action_history.len() < 2 && *variant != TakVariant::Flat {
                    return Err(InvalidActionReason::OpeningViolation);
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
                    return Err(InvalidActionReason::InvalidPlace(
                        InvalidPlaceReason::NoPiecesRemaining,
                    ));
                }
                if let Err(e) = self.board.can_do_place(pos) {
                    return Err(InvalidActionReason::InvalidPlace(e));
                }
                Ok(())
            }
            TakAction::Move { pos, dir, drops } => {
                if self.action_history.len() < 2 {
                    return Err(InvalidActionReason::OpeningViolation);
                }
                if let Err(e) = self.board.can_do_move(pos, dir, drops) {
                    return Err(InvalidActionReason::InvalidMove(e));
                }
                Ok(())
            }
        }
    }

    fn do_action(&self, action: TakAction) -> Result<TakBaseGame, InvalidActionReason> {
        if let Err(e) = self.can_do_action(&action) {
            return Err(e);
        }
        let mut new_state = self.clone();
        match &action {
            TakAction::Place { pos, variant } => {
                let placing_player = if self.action_history.len() < 2 {
                    self.current_player.opponent()
                } else {
                    self.current_player.clone()
                };
                let reserve = match new_state.current_player {
                    TakPlayer::White => &mut new_state.reserves.0,
                    TakPlayer::Black => &mut new_state.reserves.1,
                };
                let amount = match variant {
                    TakVariant::Flat | TakVariant::Standing => &mut reserve.pieces,
                    TakVariant::Capstone => &mut reserve.capstones,
                };
                *amount -= 1;
                new_state
                    .board
                    .do_place(pos, variant, &placing_player)
                    .expect("can_do_action should have prevented invalid place due to board state");
            }
            TakAction::Move { pos, dir, drops } => {
                new_state
                    .board
                    .do_move(pos, dir, drops)
                    .expect("can_do_action should have prevented invalid move due to board state");
            }
        }

        let board_hash = self.board.compute_hash_string();
        new_state
            .board_hash_history
            .entry(board_hash.clone())
            .and_modify(|e| *e += 1)
            .or_insert(1);

        match new_state.check_game_over(board_hash) {
            TakBaseGame::Finished(finished_game) => {
                return Ok(TakBaseGame::Finished(finished_game));
            }
            TakBaseGame::Ongoing(mut ongoing_game) => {
                ongoing_game.current_player = ongoing_game.current_player.opponent();
                ongoing_game.action_history.push(action);

                Ok(TakBaseGame::Ongoing(ongoing_game))
            }
        }
    }

    fn check_game_over(self, board_hash: String) -> TakBaseGame {
        let white_reserve_empty = self.reserves.0.pieces == 0 && self.reserves.0.capstones == 0;
        let black_reserve_empty = self.reserves.1.pieces == 0 && self.reserves.1.capstones == 0;

        let game_result = if self.board.check_for_road(&self.current_player) {
            Some(TakGameResult::Win {
                winner: self.current_player.clone(),
                reason: TakWinReason::Road,
            })
        } else if self.board.check_for_road(&self.current_player.opponent()) {
            Some(TakGameResult::Win {
                winner: self.current_player.opponent(),
                reason: TakWinReason::Road,
            })
        } else if self.board.is_full() || white_reserve_empty || black_reserve_empty {
            let (white_flats, black_flats) = self.board.count_flats();
            let (white_score, black_score) =
                (white_flats * 2, black_flats * 2 + self.settings.half_komi);
            Some(match white_score.cmp(&black_score) {
                std::cmp::Ordering::Greater => TakGameResult::Win {
                    winner: TakPlayer::White,
                    reason: TakWinReason::Flats,
                },
                std::cmp::Ordering::Less => TakGameResult::Win {
                    winner: TakPlayer::Black,
                    reason: TakWinReason::Flats,
                },
                std::cmp::Ordering::Equal => TakGameResult::Draw,
            })
        } else if let Some(repeat_count) = self.board_hash_history.get(&board_hash)
            && *repeat_count >= 3
        {
            Some(TakGameResult::Draw)
        } else {
            None
        };
        match game_result {
            Some(state) => TakBaseGame::Finished(TakFinishedBaseGame::new(self, state)),
            None => TakBaseGame::Ongoing(self),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TakFinishedGame {
    base: TakFinishedBaseGame,
    time_remaining: (Duration, Duration),
}

impl TakFinishedGame {
    fn new(ongoing_game: TakOngoingGame, game_result: TakGameResult) -> Self {
        TakFinishedGame {
            base: TakFinishedBaseGame::new(ongoing_game.base, game_result),
            time_remaining: ongoing_game.clock.remaining_time,
        }
    }

    fn from_finished_base(
        finished_base: TakFinishedBaseGame,
        ongoing_game: TakOngoingGame,
    ) -> Self {
        TakFinishedGame {
            base: finished_base,
            time_remaining: ongoing_game.clock.remaining_time,
        }
    }

    pub fn action_history(&self) -> &Vec<TakAction> {
        &self.base.action_history
    }

    pub fn game_result(&self) -> &TakGameResult {
        &self.base.game_result
    }

    pub fn get_time_remaining(&self) -> (Duration, Duration) {
        self.time_remaining
    }
}

#[derive(Clone, Debug)]
pub struct TakOngoingGame {
    base: TakOngoingBaseGame,
    draw_offered: (bool, bool),
    undo_requested: (bool, bool),
    clock: TakClock,
}

#[derive(Clone, Debug)]
pub struct TakClock {
    remaining_time: (Duration, Duration),
    last_update_timestamp: Instant,
    has_gained_extra_time: (bool, bool),
    is_ticking: bool,
}

pub enum TakGame {
    Ongoing(TakOngoingGame),
    Finished(TakFinishedGame),
}

impl TakOngoingGame {
    pub fn new(settings: TakGameSettings) -> Self {
        let base_game = TakOngoingBaseGame::new(settings.clone());
        TakOngoingGame {
            base: base_game,
            draw_offered: (false, false),
            undo_requested: (false, false),
            clock: TakClock {
                remaining_time: (
                    settings.time_control.contingent,
                    settings.time_control.contingent,
                ),
                last_update_timestamp: Instant::now(),
                has_gained_extra_time: (false, false),
                is_ticking: false,
            },
        }
    }

    pub fn action_history(&self) -> &Vec<TakAction> {
        &self.base.action_history
    }

    pub fn draw_offers(&self) -> (bool, bool) {
        self.draw_offered
    }

    pub fn undo_requests(&self) -> (bool, bool) {
        self.undo_requested
    }

    pub fn current_player(&self) -> TakPlayer {
        self.base.current_player
    }

    fn set_game_over(mut self, now: Instant, game_result: TakGameResult) -> TakFinishedGame {
        let player = self.base.current_player.clone();
        self.stop_clock(now, &player);
        TakFinishedGame::new(self, game_result)
    }

    pub fn check_timeout(&self, now: Instant) -> Option<TakFinishedGame> {
        let player = self.base.current_player.clone();
        let time_remaining = self.get_time_remaining(&player, now);
        if time_remaining.is_zero() {
            let game_result = TakGameResult::Win {
                winner: player.opponent(),
                reason: TakWinReason::Default,
            };
            let mut new_state = self.clone();
            new_state.stop_clock(now, &player);
            Some(TakFinishedGame::new(new_state, game_result))
        } else {
            None
        }
    }

    pub fn get_time_remaining(&self, player: &TakPlayer, now: Instant) -> Duration {
        let base_remaining = match player {
            TakPlayer::White => self.clock.remaining_time.0,
            TakPlayer::Black => self.clock.remaining_time.1,
        };
        if self.base.current_player != *player || !self.clock.is_ticking {
            return base_remaining;
        }
        let elapsed = now.saturating_duration_since(self.clock.last_update_timestamp);
        base_remaining.saturating_sub(elapsed)
    }

    pub fn get_time_remaining_both(&self, now: Instant) -> (Duration, Duration) {
        (
            self.get_time_remaining(&TakPlayer::White, now),
            self.get_time_remaining(&TakPlayer::Black, now),
        )
    }

    fn maybe_apply_elapsed(&mut self, now: Instant, player: &TakPlayer, add_increment: bool) {
        let remaining = match player {
            TakPlayer::White => &mut self.clock.remaining_time.0,
            TakPlayer::Black => &mut self.clock.remaining_time.1,
        };
        if self.clock.is_ticking {
            let elapsed = now.saturating_duration_since(self.clock.last_update_timestamp);
            *remaining = remaining.saturating_sub(elapsed);
        }
        if add_increment {
            *remaining = remaining.saturating_add(self.base.settings.time_control.increment);
        }
        self.clock.last_update_timestamp = now;
    }

    fn maybe_gain_extra_time(&mut self, player: &TakPlayer) {
        if !self.clock.is_ticking {
            return;
        }
        let time_control = &self.base.settings.time_control;
        let remaining = match player {
            TakPlayer::White => &mut self.clock.remaining_time.0,
            TakPlayer::Black => &mut self.clock.remaining_time.1,
        };
        if let Some((extra_move_index, extra_time)) = time_control.extra {
            let has_gained_extra_time = match player {
                TakPlayer::White => &mut self.clock.has_gained_extra_time.0,
                TakPlayer::Black => &mut self.clock.has_gained_extra_time.1,
            };
            // ply index is incremented before clock update, which means it is odd for white moves and starts at 1 for move 1
            // move 1: white 1, black 2 ---(+1)--> (2, 3) ---(/2)--> (1, 1)
            // move 2: white 3, black 4 ---(+1)--> (4, 5) ---(/2)--> (2, 2)
            let move_index = (self.base.action_history.len() + 1) / 2;
            if !*has_gained_extra_time && extra_move_index as usize == move_index {
                *remaining = remaining.saturating_add(extra_time);
                *has_gained_extra_time = true;
            }
        }
    }

    fn start_or_update_clock(&mut self, now: Instant, player: &TakPlayer) {
        self.maybe_apply_elapsed(now, player, true);
        self.maybe_gain_extra_time(player);

        self.clock.is_ticking = true;
    }

    fn stop_clock(&mut self, now: Instant, player: &TakPlayer) {
        self.maybe_apply_elapsed(now, player, false);

        self.clock.is_ticking = false;
    }

    pub fn do_action(
        &self,
        action: TakAction,
        now: Instant,
    ) -> Result<MaybeTimeout<TakGame>, InvalidActionReason> {
        if let Some(finished_game) = self.check_timeout(now) {
            return Ok(MaybeTimeout::Timeout(finished_game));
        };

        let player = self.base.current_player.clone();

        match self.base.do_action(action) {
            Ok(TakBaseGame::Ongoing(new_base)) => {
                let mut new_state = self.clone();
                new_state.base = new_base;
                new_state.start_or_update_clock(now, &player);
                Ok(MaybeTimeout::Result(TakGame::Ongoing(new_state)))
            }
            Ok(TakBaseGame::Finished(finished_base)) => {
                let mut new_state = self.clone();
                new_state.stop_clock(now, &player);
                let finished_game = TakFinishedGame::from_finished_base(finished_base, new_state);
                Ok(MaybeTimeout::Result(TakGame::Finished(finished_game)))
            }
            Err(e) => Err(e),
        }
    }

    //TODO: maybe only accept request if there is a move to undo
    fn undo_action(&mut self, now: Instant) -> bool {
        if self.base.action_history.pop().is_none() {
            return false;
        };
        let player = self.base.current_player.clone();
        let mut game_clone = TakOngoingBaseGame::new(self.base.settings.clone());
        for record in &self.base.action_history {
            match game_clone.do_action(record.clone()) {
                Ok(TakBaseGame::Ongoing(ongoing_base_game)) => game_clone = ongoing_base_game,
                Ok(TakBaseGame::Finished(_)) => {
                    //This should never happen, and the module is closed to preserve invariants, so we panic here
                    panic!(
                        "Finished game encountered when replaying action during undo: {:?}",
                        record
                    );
                }
                Err(e) => {
                    //This should never happen, and the module is closed to preserve invariants, so we panic here
                    panic!(
                        "Failed to replay action during undo: {:?}, error: {:?}",
                        record, e
                    );
                }
            }
        }
        self.base = game_clone;
        self.maybe_apply_elapsed(now, &player, true); // TODO: confirm that undo is supposed to add increment
        true
    }

    pub fn give_time_to_player(
        &self,
        player: &TakPlayer,
        duration: Duration,
        now: Instant,
    ) -> MaybeTimeout<TakOngoingGame> {
        if let Some(finished_game) = self.check_timeout(now) {
            return MaybeTimeout::Timeout(finished_game);
        };
        let mut new_state = self.clone();
        let remaining = match player {
            TakPlayer::White => &mut new_state.clock.remaining_time.0,
            TakPlayer::Black => &mut new_state.clock.remaining_time.1,
        };
        *remaining = remaining.saturating_add(duration);
        MaybeTimeout::Result(new_state)
    }

    pub fn resign(&self, player: &TakPlayer, now: Instant) -> MaybeTimeout<TakFinishedGame> {
        if let Some(finished_game) = self.check_timeout(now) {
            return MaybeTimeout::Timeout(finished_game);
        };
        MaybeTimeout::Result(self.clone().set_game_over(
            now,
            TakGameResult::Win {
                winner: player.opponent(),
                reason: TakWinReason::Default,
            },
        ))
    }

    pub fn offer_draw(
        &self,
        player: &TakPlayer,
        offer: bool,
        now: Instant,
    ) -> MaybeTimeout<Option<TakGame>> {
        if let Some(finished_game) = self.check_timeout(now) {
            return MaybeTimeout::Timeout(finished_game);
        };
        let mut new_state = self.clone();

        let current_offer = match player {
            TakPlayer::White => &mut new_state.draw_offered.0,
            TakPlayer::Black => &mut new_state.draw_offered.1,
        };
        if *current_offer == offer {
            return MaybeTimeout::Result(None);
        }
        *current_offer = offer;

        if new_state.draw_offered.0 && new_state.draw_offered.1 {
            let finished_game = new_state.set_game_over(now, TakGameResult::Draw);
            MaybeTimeout::Result(Some(TakGame::Finished(finished_game)))
        } else {
            MaybeTimeout::Result(Some(TakGame::Ongoing(new_state)))
        }
    }

    /// Request an undo. If both players have requested an undo, the last action is undone.
    /// If the game is finished due to timeout, returns the finished game.
    /// If there are no actions to undo, returns None.
    pub fn request_undo(
        &self,
        player: &TakPlayer,
        request: bool,
        now: Instant,
    ) -> MaybeTimeout<Option<TakOngoingGame>> {
        if let Some(finished_game) = self.check_timeout(now) {
            return MaybeTimeout::Timeout(finished_game);
        };
        let mut new_state = self.clone();

        let current_request = match player {
            TakPlayer::White => &mut new_state.undo_requested.0,
            TakPlayer::Black => &mut new_state.undo_requested.1,
        };
        if *current_request == request {
            return MaybeTimeout::Result(None);
        }
        *current_request = request;

        if new_state.undo_requested.0 && new_state.undo_requested.1 {
            new_state.undo_requested = (false, false);
            new_state.undo_action(now); // For now, we ignore if undo_action fails (no action to undo)
        }
        MaybeTimeout::Result(Some(new_state))
    }
}

#[cfg(test)]
mod tests {
    use crate::{TakDir, TakPos, TakTimeControl};

    use super::*;

    fn do_move(game: &mut TakOngoingGame, action: TakAction, now: Instant) {
        match game.do_action(action, now) {
            Ok(MaybeTimeout::Timeout(_)) => {
                panic!("Game finished unexpectedly due to timeout")
            }
            Ok(MaybeTimeout::Result(TakGame::Ongoing(g))) => *game = g,
            Ok(MaybeTimeout::Result(TakGame::Finished(_))) => {
                panic!("Game finished unexpectedly")
            }
            Err(e) => panic!("Failed to do action: {:?}", e),
        }
    }

    fn do_finish_move(
        game: &mut TakOngoingGame,
        action: TakAction,
        now: Instant,
        expected_result: TakGameResult,
    ) {
        match game.do_action(action, now) {
            Ok(MaybeTimeout::Result(TakGame::Ongoing(_))) => {
                panic!("Game should have finished, but is ongoing")
            }
            Ok(MaybeTimeout::Timeout(_)) => {
                panic!("Game finished unexpectedly due to timeout")
            }
            Ok(MaybeTimeout::Result(TakGame::Finished(g))) => {
                assert_eq!(g.base.game_result, expected_result)
            }
            Err(e) => panic!("Failed to do action: {:?}", e),
        }
    }

    #[test]
    fn test_reserve_constraints() {
        let now = Instant::now();
        let mut game = TakOngoingGame::new(TakGameSettings {
            board_size: 5,
            half_komi: 0,
            reserve: TakReserve::new(3, 1),
            time_control: TakTimeControl {
                contingent: Duration::from_secs(300),
                increment: Duration::from_secs(5),
                extra: Some((2, Duration::from_secs(60))),
            },
        });

        do_move(
            &mut game,
            TakAction::Place {
                pos: TakPos::new(0, 0),
                variant: TakVariant::Flat,
            },
            now,
        );
        do_move(
            &mut game,
            TakAction::Place {
                pos: TakPos::new(1, 0),
                variant: TakVariant::Flat,
            },
            now,
        );
        do_move(
            &mut game,
            TakAction::Place {
                pos: TakPos::new(2, 0),
                variant: TakVariant::Flat,
            },
            now,
        );
        do_move(
            &mut game,
            TakAction::Place {
                pos: TakPos::new(3, 0),
                variant: TakVariant::Capstone,
            },
            now,
        );
        do_move(
            &mut game,
            TakAction::Place {
                pos: TakPos::new(4, 0),
                variant: TakVariant::Flat,
            },
            now,
        );

        // player 2 has placed one flat and one capstone, has two flat left
        assert!(
            game.do_action(
                TakAction::Place {
                    pos: TakPos::new(0, 1),
                    variant: TakVariant::Capstone,
                },
                now
            )
            .is_err()
        );
        do_move(
            &mut game,
            TakAction::Place {
                pos: TakPos::new(0, 1),
                variant: TakVariant::Flat,
            },
            now,
        );

        // player 1 has placed all flats and has a capstone left
        assert!(
            game.do_action(
                TakAction::Place {
                    pos: TakPos::new(0, 2),
                    variant: TakVariant::Flat,
                },
                now
            )
            .is_err()
        );

        // game should be over now
        // player 1 wins with 2 flats against 1 flat
        do_finish_move(
            &mut game,
            TakAction::Place {
                pos: TakPos::new(0, 2),
                variant: TakVariant::Capstone,
            },
            now,
            TakGameResult::Win {
                winner: TakPlayer::White,
                reason: TakWinReason::Flats,
            },
        );
    }

    #[test]
    fn test_komi_effect() {
        let now = Instant::now();
        for (half_komi, result, result2) in [
            (
                0,
                TakGameResult::Win {
                    winner: TakPlayer::White,
                    reason: TakWinReason::Flats,
                },
                TakGameResult::Draw,
            ),
            (
                1,
                TakGameResult::Win {
                    winner: TakPlayer::White,
                    reason: TakWinReason::Flats,
                },
                TakGameResult::Win {
                    winner: TakPlayer::Black,
                    reason: TakWinReason::Flats,
                },
            ),
            (
                2,
                TakGameResult::Draw,
                TakGameResult::Win {
                    winner: TakPlayer::Black,
                    reason: TakWinReason::Flats,
                },
            ),
            (
                3,
                TakGameResult::Win {
                    winner: TakPlayer::Black,
                    reason: TakWinReason::Flats,
                },
                TakGameResult::Win {
                    winner: TakPlayer::Black,
                    reason: TakWinReason::Flats,
                },
            ),
        ] {
            let mut game = TakOngoingGame::new(TakGameSettings {
                board_size: 5,
                half_komi,
                reserve: TakReserve::new(2, 0),
                time_control: TakTimeControl {
                    contingent: Duration::from_secs(300),
                    increment: Duration::from_secs(5),
                    extra: Some((2, Duration::from_secs(60))),
                },
            });
            do_move(
                &mut game,
                TakAction::Place {
                    pos: TakPos::new(0, 0),
                    variant: TakVariant::Flat,
                },
                now,
            );
            do_move(
                &mut game,
                TakAction::Place {
                    pos: TakPos::new(1, 0),
                    variant: TakVariant::Flat,
                },
                now,
            );
            do_finish_move(
                &mut game,
                TakAction::Place {
                    pos: TakPos::new(2, 0),
                    variant: TakVariant::Flat,
                },
                now,
                result,
            );

            let mut game2 = TakOngoingGame::new(TakGameSettings {
                board_size: 5,
                half_komi,
                reserve: TakReserve::new(1, 1),
                time_control: TakTimeControl {
                    contingent: Duration::from_secs(300),
                    increment: Duration::from_secs(5),
                    extra: Some((2, Duration::from_secs(60))),
                },
            });
            do_move(
                &mut game,
                TakAction::Place {
                    pos: TakPos::new(0, 0),
                    variant: TakVariant::Flat,
                },
                now,
            );
            do_move(
                &mut game2,
                TakAction::Place {
                    pos: TakPos::new(1, 0),
                    variant: TakVariant::Flat,
                },
                now,
            );
            do_finish_move(
                &mut game2,
                TakAction::Place {
                    pos: TakPos::new(2, 0),
                    variant: TakVariant::Capstone,
                },
                now,
                result2,
            );
        }
    }

    #[test]
    fn test_first_move_must_be_flat() {
        let now = Instant::now();
        let mut game = TakOngoingGame::new(TakGameSettings {
            board_size: 5,
            half_komi: 0,
            reserve: TakReserve::new(21, 1),
            time_control: TakTimeControl {
                contingent: Duration::from_secs(300),
                increment: Duration::from_secs(5),
                extra: Some((2, Duration::from_secs(60))),
            },
        });

        // first move must be flat stone
        assert!(
            game.do_action(
                TakAction::Place {
                    pos: TakPos::new(0, 0),
                    variant: TakVariant::Capstone,
                },
                now
            )
            .is_err()
        );
        assert!(
            game.do_action(
                TakAction::Place {
                    pos: TakPos::new(0, 0),
                    variant: TakVariant::Standing,
                },
                now
            )
            .is_err()
        );
        do_move(
            &mut game,
            TakAction::Place {
                pos: TakPos::new(0, 0),
                variant: TakVariant::Flat,
            },
            now,
        );

        // second move must be flat stone
        assert!(
            game.do_action(
                TakAction::Place {
                    pos: TakPos::new(1, 0),
                    variant: TakVariant::Capstone,
                },
                now
            )
            .is_err()
        );
        assert!(
            game.do_action(
                TakAction::Place {
                    pos: TakPos::new(1, 0),
                    variant: TakVariant::Standing,
                },
                now
            )
            .is_err()
        );
        // moving piece from first place is not allowed either
        assert!(
            game.do_action(
                TakAction::Move {
                    pos: TakPos::new(0, 0),
                    dir: TakDir::Right,
                    drops: vec![1],
                },
                now
            )
            .is_err()
        );

        do_move(
            &mut game,
            TakAction::Place {
                pos: TakPos::new(1, 0),
                variant: TakVariant::Flat,
            },
            now,
        );

        // from third move onwards, any variant is allowed
        do_move(
            &mut game,
            TakAction::Place {
                pos: TakPos::new(0, 1),
                variant: TakVariant::Capstone,
            },
            now,
        );
        do_move(
            &mut game,
            TakAction::Place {
                pos: TakPos::new(0, 2),
                variant: TakVariant::Standing,
            },
            now,
        );
        do_move(
            &mut game,
            TakAction::Place {
                pos: TakPos::new(0, 3),
                variant: TakVariant::Flat,
            },
            now,
        );
    }
}
