use std::time::{Duration, Instant, SystemTime};

use crate::{
    InvalidActionReason, MaybeTimeout, TakAction, TakActionRecord, TakGameResult, TakGameSettings,
    TakPlayer, TakTimeInfo, TakTimeSettings, TakWinReason, base::TakOngoingBaseGame,
};

#[derive(Clone, Debug)]
pub struct TakFinishedGame {
    pub result: TakGameResult,
    pub time_info: TakTimeInfo,
    pub history: Vec<TakActionRecord>,
}

impl TakFinishedGame {
    fn new(
        game_result: TakGameResult,
        time_info: TakTimeInfo,
        history: Vec<TakActionRecord>,
    ) -> Self {
        TakFinishedGame {
            result: game_result,
            time_info,
            history,
        }
    }
}

pub trait TakInstant: Copy {
    fn saturating_duration_since(&self, earlier: Self) -> Duration;
}

impl TakInstant for Instant {
    fn saturating_duration_since(&self, earlier: Self) -> Duration {
        self.saturating_duration_since(earlier)
    }
}

impl TakInstant for SystemTime {
    fn saturating_duration_since(&self, earlier: Self) -> Duration {
        self.duration_since(earlier).unwrap_or(Duration::ZERO)
    }
}

impl TakInstant for () {
    fn saturating_duration_since(&self, _earlier: Self) -> Duration {
        Duration::ZERO
    }
}

#[derive(Clone, Debug)]
pub struct TakClock<T: TakInstant> {
    pub remaining_time: TakTimeInfo,
    pub last_update_timestamp: T,
    pub is_ticking: bool,
}

impl<T: TakInstant> TakClock<T> {
    fn end_turn(
        &mut self,
        settings: &TakTimeSettings,
        game: &TakOngoingBaseGame,
        player: TakPlayer,
    ) {
        match settings {
            TakTimeSettings::Realtime(settings) => {
                let remaining = match player {
                    TakPlayer::White => &mut self.remaining_time.white_remaining,
                    TakPlayer::Black => &mut self.remaining_time.black_remaining,
                };
                *remaining = remaining.saturating_add(settings.increment);

                if let Some((extra_move_index, extra_time)) = settings.extra {
                    // ply index is incremented before clock update, which means it is odd for white moves and starts at 1 for move 1
                    // move 1: white 1, black 2 ---(+1)--> (2, 3) ---(/2)--> (1, 1)
                    // move 2: white 3, black 4 ---(+1)--> (4, 5) ---(/2)--> (2, 2)
                    let move_index = (game.ply_index + 1) / 2;
                    if extra_move_index as usize == move_index {
                        *remaining = remaining.saturating_add(extra_time);
                    }
                }
            }
            TakTimeSettings::Async(settings) => {
                self.remaining_time = TakTimeInfo {
                    white_remaining: settings.contingent,
                    black_remaining: settings.contingent,
                };
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct TakOngoingGame<T: TakInstant> {
    base: TakOngoingBaseGame,
    pub clock: TakClock<T>,
    pub time_settings: TakTimeSettings,
    pub history: Vec<TakActionRecord>,
}

impl<T: TakInstant> TakOngoingGame<T> {
    pub fn new(settings: &TakGameSettings, now: T) -> Self {
        let base_game = TakOngoingBaseGame::new(&settings.base);
        let contingent = match &settings.time_settings {
            TakTimeSettings::Realtime(settings) => settings.contingent,
            TakTimeSettings::Async(settings) => settings.contingent,
        };
        let clock = TakClock {
            remaining_time: TakTimeInfo {
                white_remaining: contingent,
                black_remaining: contingent,
            },
            last_update_timestamp: now,
            is_ticking: false,
        };

        Self {
            base: base_game,
            clock,
            time_settings: settings.time_settings.clone(),
            history: Vec::new(),
        }
    }
    pub fn from_history(
        settings: &TakGameSettings,
        clock: TakClock<T>,
        history: Vec<TakActionRecord>,
    ) -> Result<Self, InvalidActionReason> {
        let mut base = TakOngoingBaseGame::new(&settings.base);
        for record in &history {
            base.do_action(record.action.clone())?;
        }
        Ok(Self {
            base,
            clock,
            time_settings: settings.time_settings.clone(),
            history,
        })
    }

    pub fn iter_actions(&self) -> impl Iterator<Item = &TakAction> {
        self.history.iter().map(|record| &record.action)
    }

    pub fn ply_index(&self) -> usize {
        self.base.ply_index
    }

    pub fn current_player(&self) -> TakPlayer {
        self.base.current_player
    }

    fn set_game_over(&mut self, now: T, game_result: TakGameResult) -> TakFinishedGame {
        let player = self.base.current_player;
        self.stop_clock(now, player);
        TakFinishedGame::new(game_result, self.get_time_info(now), self.history.clone())
    }

    pub fn check_timeout(&mut self, now: T) -> MaybeTimeout<(), TakFinishedGame> {
        let player = self.base.current_player;
        let time_remaining = self.get_time_remaining(player, now);
        if time_remaining.is_zero() {
            let game_result = TakGameResult::Win {
                winner: player.opponent(),
                reason: TakWinReason::Default,
            };
            let finished_game = self.set_game_over(now, game_result);
            MaybeTimeout::Timeout(finished_game)
        } else {
            MaybeTimeout::Result(())
        }
    }

    fn get_time_remaining(&self, player: TakPlayer, now: T) -> Duration {
        let base_remaining = match player {
            TakPlayer::White => self.clock.remaining_time.white_remaining,
            TakPlayer::Black => self.clock.remaining_time.black_remaining,
        };
        if self.base.current_player != player || !self.clock.is_ticking {
            return base_remaining;
        }
        let elapsed = now.saturating_duration_since(self.clock.last_update_timestamp);
        base_remaining.saturating_sub(elapsed)
    }

    pub fn get_time_info(&self, now: T) -> TakTimeInfo {
        TakTimeInfo {
            white_remaining: self.get_time_remaining(TakPlayer::White, now),
            black_remaining: self.get_time_remaining(TakPlayer::Black, now),
        }
    }

    fn maybe_apply_elapsed(&mut self, now: T, player: TakPlayer) {
        let remaining = match player {
            TakPlayer::White => &mut self.clock.remaining_time.white_remaining,
            TakPlayer::Black => &mut self.clock.remaining_time.black_remaining,
        };
        if self.clock.is_ticking {
            let elapsed = now.saturating_duration_since(self.clock.last_update_timestamp);
            *remaining = remaining.saturating_sub(elapsed);
        }

        self.clock.last_update_timestamp = now;
    }

    fn start_or_update_clock(&mut self, now: T, player: TakPlayer) {
        self.maybe_apply_elapsed(now, player);
        self.clock.end_turn(&self.time_settings, &self.base, player);
        self.clock.is_ticking = true;
    }

    fn stop_clock(&mut self, now: T, player: TakPlayer) {
        self.maybe_apply_elapsed(now, player);
        self.clock.is_ticking = false;
    }

    pub fn do_action(
        &mut self,
        action: TakAction,
        now: T,
    ) -> Result<MaybeTimeout<Option<TakFinishedGame>, TakFinishedGame>, InvalidActionReason> {
        if let MaybeTimeout::Timeout(finished_game) = self.check_timeout(now) {
            return Ok(MaybeTimeout::Timeout(finished_game));
        };

        let player = self.base.current_player;
        let remaining_before_action = self.get_time_info(now);
        let record = TakActionRecord {
            action: action.clone(),
            time_info: remaining_before_action,
        };

        match self.base.do_action(action)? {
            None => {
                self.start_or_update_clock(now, player);
                self.history.push(record);
                Ok(MaybeTimeout::Result(None))
            }
            Some(result) => {
                self.stop_clock(now, player);
                self.history.push(record);
                let finished_game =
                    TakFinishedGame::new(result, self.get_time_info(now), self.history.clone());
                Ok(MaybeTimeout::Result(Some(finished_game)))
            }
        }
    }

    pub fn undo_action(
        &mut self,
        now: T,
    ) -> Result<MaybeTimeout<bool, TakFinishedGame>, InvalidActionReason> {
        if let MaybeTimeout::Timeout(finished_game) = self.check_timeout(now) {
            return Ok(MaybeTimeout::Timeout(finished_game));
        };
        let Some(record) = self.history.pop() else {
            return Ok(MaybeTimeout::Result(false));
        };

        let mut new_base = TakOngoingBaseGame::new(&self.base.settings);
        for record in &self.history {
            new_base.do_action(record.action.clone())?;
        }
        self.clock.remaining_time = record.time_info;
        Ok(MaybeTimeout::Result(true))
    }

    pub fn resign_or_abandon(
        &mut self,
        player: TakPlayer,
        now: T,
    ) -> MaybeTimeout<TakFinishedGame, TakFinishedGame> {
        if let MaybeTimeout::Timeout(finished_game) = self.check_timeout(now) {
            return MaybeTimeout::Timeout(finished_game);
        };
        MaybeTimeout::Result(self.set_game_over(
            now,
            TakGameResult::Win {
                winner: player.opponent(),
                reason: TakWinReason::Default,
            },
        ))
    }

    pub fn try_abort(&mut self, now: T) -> MaybeTimeout<Option<TakFinishedGame>, TakFinishedGame> {
        if let MaybeTimeout::Timeout(finished_game) = self.check_timeout(now) {
            return MaybeTimeout::Timeout(finished_game);
        };
        if self.base.ply_index <= 1 {
            // if one player hasn't made a move yet, both players can abort without penalty
            return MaybeTimeout::Result(Some(self.set_game_over(now, TakGameResult::Aborted)));
        }
        MaybeTimeout::Result(None)
    }

    pub fn abort_forced(&mut self, now: T) -> MaybeTimeout<TakFinishedGame, TakFinishedGame> {
        if let MaybeTimeout::Timeout(finished_game) = self.check_timeout(now) {
            return MaybeTimeout::Timeout(finished_game);
        };
        MaybeTimeout::Result(self.set_game_over(now, TakGameResult::Aborted))
    }

    pub fn agree_draw(&mut self, now: T) -> MaybeTimeout<TakFinishedGame, TakFinishedGame> {
        if let MaybeTimeout::Timeout(finished_game) = self.check_timeout(now) {
            return MaybeTimeout::Timeout(finished_game);
        };
        MaybeTimeout::Result(self.set_game_over(now, TakGameResult::Draw))
    }

    pub fn give_time_to_player(
        &mut self,
        player: TakPlayer,
        duration: Duration,
        now: T,
    ) -> MaybeTimeout<(), TakFinishedGame> {
        if let MaybeTimeout::Timeout(finished_game) = self.check_timeout(now) {
            return MaybeTimeout::Timeout(finished_game);
        };

        let remaining = match player {
            TakPlayer::White => &mut self.clock.remaining_time.white_remaining,
            TakPlayer::Black => &mut self.clock.remaining_time.black_remaining,
        };
        *remaining = remaining.saturating_add(duration);

        MaybeTimeout::Result(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        TakBaseGameSettings, TakDir, TakOpening, TakPos, TakRealtimeTimeControl, TakReserve,
        TakVariant,
    };

    use super::*;

    fn do_move<T: TakInstant>(game: &mut TakOngoingGame<T>, action: TakAction, now: T) {
        match game.do_action(action, now) {
            Ok(MaybeTimeout::Timeout(_)) => {
                panic!("Game finished unexpectedly due to timeout")
            }
            Ok(MaybeTimeout::Result(None)) => {}
            Ok(MaybeTimeout::Result(Some(_))) => {
                panic!("Game finished unexpectedly")
            }
            Err(e) => panic!("Failed to do action: {:?}", e),
        }
    }

    fn do_finish_move<T: TakInstant>(
        game: &mut TakOngoingGame<T>,
        action: TakAction,
        now: T,
        expected_result: TakGameResult,
    ) {
        match game.do_action(action, now) {
            Ok(MaybeTimeout::Result(None)) => {
                panic!("Game should have finished, but is ongoing")
            }
            Ok(MaybeTimeout::Timeout(_)) => {
                panic!("Game finished unexpectedly due to timeout")
            }
            Ok(MaybeTimeout::Result(Some(g))) => {
                assert_eq!(g.result, expected_result)
            }
            Err(e) => panic!("Failed to do action: {:?}", e),
        }
    }

    #[test]
    fn test_reserve_constraints() {
        let now = Instant::now();
        let mut game = TakOngoingGame::new(
            &TakGameSettings {
                base: TakBaseGameSettings {
                    board_size: 5,
                    half_komi: 0,
                    reserve: TakReserve::new(3, 1),
                    opening: TakOpening::Swap,
                },
                time_settings: TakTimeSettings::Realtime(TakRealtimeTimeControl {
                    contingent: Duration::from_secs(300),
                    increment: Duration::from_secs(5),
                    extra: Some((2, Duration::from_secs(60))),
                }),
            },
            now,
        );

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
            let mut game = TakOngoingGame::new(
                &TakGameSettings {
                    base: TakBaseGameSettings {
                        board_size: 5,
                        half_komi,
                        reserve: TakReserve::new(2, 0),
                        opening: TakOpening::Swap,
                    },
                    time_settings: TakTimeSettings::Realtime(TakRealtimeTimeControl {
                        contingent: Duration::from_secs(300),
                        increment: Duration::from_secs(5),
                        extra: Some((2, Duration::from_secs(60))),
                    }),
                },
                now,
            );
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

            let mut game2 = TakOngoingGame::new(
                &TakGameSettings {
                    base: TakBaseGameSettings {
                        board_size: 5,
                        half_komi,
                        reserve: TakReserve::new(1, 1),
                        opening: TakOpening::Swap,
                    },
                    time_settings: TakTimeSettings::Realtime(TakRealtimeTimeControl {
                        contingent: Duration::from_secs(300),
                        increment: Duration::from_secs(5),
                        extra: Some((2, Duration::from_secs(60))),
                    }),
                },
                now,
            );
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
        let mut game = TakOngoingGame::new(
            &TakGameSettings {
                base: TakBaseGameSettings {
                    board_size: 5,
                    half_komi: 0,
                    reserve: TakReserve::new(21, 1),
                    opening: TakOpening::Swap,
                },
                time_settings: TakTimeSettings::Realtime(TakRealtimeTimeControl {
                    contingent: Duration::from_secs(300),
                    increment: Duration::from_secs(5),
                    extra: Some((2, Duration::from_secs(60))),
                }),
            },
            now,
        );

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
