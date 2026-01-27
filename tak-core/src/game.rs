use std::time::{Duration, Instant};

use crate::{
    InvalidActionReason, MaybeTimeout, TakAction, TakAsyncTimeControl, TakGameResult,
    TakGameSettings, TakPlayer, TakRealtimeTimeControl, TakTimeInfo, TakTimeSettings, TakWinReason,
    base::{TakFinishedBaseGame, TakOngoingBaseGame},
};

#[derive(Clone, Debug)]
pub struct TakFinishedGame {
    base: TakFinishedBaseGame,
    time_info: TakTimeInfo,
}

impl TakFinishedGame {
    fn new(ongoing_game: &TakOngoingGame, game_result: TakGameResult, now: Instant) -> Self {
        TakFinishedGame {
            base: TakFinishedBaseGame::new(&ongoing_game.base, game_result),
            time_info: ongoing_game.get_time_info(now),
        }
    }

    fn from_finished_base(
        finished_base: TakFinishedBaseGame,
        ongoing_game: &TakOngoingGame,
        now: Instant,
    ) -> Self {
        TakFinishedGame {
            base: finished_base,
            time_info: ongoing_game.get_time_info(now),
        }
    }

    pub fn action_history(&self) -> &Vec<TakAction> {
        &self.base.action_history
    }

    pub fn game_result(&self) -> &TakGameResult {
        &self.base.game_result
    }

    pub fn get_time_info(&self) -> TakTimeInfo {
        self.time_info.clone()
    }
}

#[derive(Clone, Debug)]
enum TakClockUpdatePolicy {
    Realtime(TakRealtimeClockUpdatePolicy),
    Async(TakAsyncClockUpdatePolicy),
}

impl TakClockUpdatePolicy {
    fn end_turn(&mut self, game: &TakOngoingBaseGame, clock: &mut TakClock, player: &TakPlayer) {
        match self {
            TakClockUpdatePolicy::Realtime(policy) => {
                policy.end_turn(game, clock, player);
            }
            TakClockUpdatePolicy::Async(policy) => {
                policy.end_turn(clock);
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct TakOngoingGame {
    base: TakOngoingBaseGame,
    clock: TakClock,
    clock_update_policy: TakClockUpdatePolicy,
}

#[derive(Clone, Debug)]
struct TakClock {
    remaining_time: (Duration, Duration),
    last_update_timestamp: Instant,
    is_ticking: bool,
}

#[derive(Clone, Debug)]
struct TakRealtimeClockUpdatePolicy {
    settings: TakRealtimeTimeControl,
    has_gained_extra_time: (bool, bool),
}

impl TakRealtimeClockUpdatePolicy {
    fn end_turn(&mut self, game: &TakOngoingBaseGame, clock: &mut TakClock, player: &TakPlayer) {
        let remaining = match player {
            TakPlayer::White => &mut clock.remaining_time.0,
            TakPlayer::Black => &mut clock.remaining_time.1,
        };
        *remaining = remaining.saturating_add(self.settings.increment);

        if let Some((extra_move_index, extra_time)) = self.settings.extra {
            let has_gained_extra_time = match player {
                TakPlayer::White => &mut self.has_gained_extra_time.0,
                TakPlayer::Black => &mut self.has_gained_extra_time.1,
            };
            // ply index is incremented before clock update, which means it is odd for white moves and starts at 1 for move 1
            // move 1: white 1, black 2 ---(+1)--> (2, 3) ---(/2)--> (1, 1)
            // move 2: white 3, black 4 ---(+1)--> (4, 5) ---(/2)--> (2, 2)
            let move_index = (game.action_history.len() + 1) / 2;
            if !*has_gained_extra_time && extra_move_index as usize == move_index {
                *remaining = remaining.saturating_add(extra_time);
                *has_gained_extra_time = true;
            }
        }
    }
}

#[derive(Clone, Debug)]
struct TakAsyncClockUpdatePolicy {
    settings: TakAsyncTimeControl,
}

impl TakAsyncClockUpdatePolicy {
    fn end_turn(&mut self, clock: &mut TakClock) {
        clock.remaining_time = (self.settings.contingent, self.settings.contingent);
    }
}

impl TakOngoingGame {
    pub fn new(settings: TakGameSettings) -> Self {
        let base_game = TakOngoingBaseGame::new(settings.base);
        let clock = match &settings.time_settings {
            TakTimeSettings::Realtime(settings) => TakClock {
                remaining_time: (settings.contingent, settings.contingent),
                last_update_timestamp: Instant::now(),
                is_ticking: false,
            },
            TakTimeSettings::Async(settings) => TakClock {
                remaining_time: (settings.contingent, settings.contingent),
                last_update_timestamp: Instant::now(),
                is_ticking: false,
            },
        };
        let mode = match &settings.time_settings {
            TakTimeSettings::Realtime(t) => {
                TakClockUpdatePolicy::Realtime(TakRealtimeClockUpdatePolicy {
                    has_gained_extra_time: (false, false),
                    settings: t.clone(),
                })
            }
            TakTimeSettings::Async(t) => TakClockUpdatePolicy::Async(TakAsyncClockUpdatePolicy {
                settings: t.clone(),
            }),
        };

        TakOngoingGame {
            base: base_game,
            clock,
            clock_update_policy: mode,
        }
    }

    pub fn action_history(&self) -> &Vec<TakAction> {
        &self.base.action_history
    }

    pub fn current_player(&self) -> TakPlayer {
        self.base.current_player
    }

    fn set_game_over(&mut self, now: Instant, game_result: TakGameResult) -> TakFinishedGame {
        let player = self.base.current_player.clone();
        self.stop_clock(now, &player);
        TakFinishedGame::new(self, game_result, now)
    }

    pub fn check_timeout(&mut self, now: Instant) -> MaybeTimeout<(), TakFinishedGame> {
        let player = self.base.current_player.clone();
        let time_remaining = self.get_time_remaining(&player, now);
        if time_remaining.is_zero() {
            let game_result = TakGameResult::Win {
                winner: player.opponent(),
                reason: TakWinReason::Default,
            };
            self.stop_clock(now, &player);
            MaybeTimeout::Timeout(TakFinishedGame::new(self, game_result, now))
        } else {
            MaybeTimeout::Result(())
        }
    }

    fn get_time_remaining(&self, player: &TakPlayer, now: Instant) -> Duration {
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

    pub fn get_time_info(&self, now: Instant) -> TakTimeInfo {
        TakTimeInfo {
            white_remaining: self.get_time_remaining(&TakPlayer::White, now),
            black_remaining: self.get_time_remaining(&TakPlayer::Black, now),
        }
    }

    fn maybe_apply_elapsed(&mut self, now: Instant, player: &TakPlayer) {
        let remaining = match player {
            TakPlayer::White => &mut self.clock.remaining_time.0,
            TakPlayer::Black => &mut self.clock.remaining_time.1,
        };
        if self.clock.is_ticking {
            let elapsed = now.saturating_duration_since(self.clock.last_update_timestamp);
            *remaining = remaining.saturating_sub(elapsed);
        }

        self.clock.last_update_timestamp = now;
    }

    fn start_or_update_clock(&mut self, now: Instant, player: &TakPlayer) {
        self.maybe_apply_elapsed(now, player);
        self.clock_update_policy
            .end_turn(&self.base, &mut self.clock, player);
        self.clock.is_ticking = true;
    }

    fn stop_clock(&mut self, now: Instant, player: &TakPlayer) {
        self.maybe_apply_elapsed(now, player);
        self.clock.is_ticking = false;
    }

    pub fn do_action(
        &mut self,
        action: TakAction,
        now: Instant,
    ) -> Result<MaybeTimeout<Option<TakFinishedGame>, TakFinishedGame>, InvalidActionReason> {
        if let MaybeTimeout::Timeout(finished_game) = self.check_timeout(now) {
            return Ok(MaybeTimeout::Timeout(finished_game));
        };

        let player = self.base.current_player.clone();

        match self.base.do_action(action) {
            Ok(None) => {
                self.start_or_update_clock(now, &player);
                Ok(MaybeTimeout::Result(None))
            }
            Ok(Some(finished_base)) => {
                self.stop_clock(now, &player);
                let finished_game = TakFinishedGame::from_finished_base(finished_base, self, now);
                Ok(MaybeTimeout::Result(Some(finished_game)))
            }
            Err(e) => Err(e),
        }
    }

    pub fn undo_action(&mut self, now: Instant) -> MaybeTimeout<bool, TakFinishedGame> {
        if let MaybeTimeout::Timeout(finished_game) = self.check_timeout(now) {
            return MaybeTimeout::Timeout(finished_game);
        };

        if self.base.action_history.pop().is_none() {
            return MaybeTimeout::Result(false);
        };
        let player = self.base.current_player.clone();
        let mut game_clone = TakOngoingBaseGame::new(self.base.settings.clone());
        for record in &self.base.action_history {
            match game_clone.do_action(record.clone()) {
                Ok(None) => {}
                Ok(Some(_)) => {
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
        self.start_or_update_clock(now, &player); // TODO: verify that we want increment after undo

        MaybeTimeout::Result(true)
    }

    pub fn resign_or_abandon(
        &mut self,
        player: &TakPlayer,
        now: Instant,
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

    pub fn agree_draw(&mut self, now: Instant) -> MaybeTimeout<TakFinishedGame, TakFinishedGame> {
        if let MaybeTimeout::Timeout(finished_game) = self.check_timeout(now) {
            return MaybeTimeout::Timeout(finished_game);
        };
        MaybeTimeout::Result(self.set_game_over(now, TakGameResult::Draw))
    }

    pub fn give_time_to_player(
        &mut self,
        player: &TakPlayer,
        duration: Duration,
        now: Instant,
    ) -> MaybeTimeout<(), TakFinishedGame> {
        if let MaybeTimeout::Timeout(finished_game) = self.check_timeout(now) {
            return MaybeTimeout::Timeout(finished_game);
        };

        let remaining = match player {
            TakPlayer::White => &mut self.clock.remaining_time.0,
            TakPlayer::Black => &mut self.clock.remaining_time.1,
        };
        *remaining = remaining.saturating_add(duration);

        MaybeTimeout::Result(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        TakBaseGameSettings, TakDir, TakPos, TakRealtimeTimeControl, TakReserve, TakVariant,
    };

    use super::*;

    fn do_move(game: &mut TakOngoingGame, action: TakAction, now: Instant) {
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

    fn do_finish_move(
        game: &mut TakOngoingGame,
        action: TakAction,
        now: Instant,
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
                assert_eq!(g.base.game_result, expected_result)
            }
            Err(e) => panic!("Failed to do action: {:?}", e),
        }
    }

    #[test]
    fn test_reserve_constraints() {
        let now = Instant::now();
        let mut game = TakOngoingGame::new(TakGameSettings {
            base: TakBaseGameSettings {
                board_size: 5,
                half_komi: 0,
                reserve: TakReserve::new(3, 1),
            },
            time_settings: TakTimeSettings::Realtime(TakRealtimeTimeControl {
                contingent: Duration::from_secs(300),
                increment: Duration::from_secs(5),
                extra: Some((2, Duration::from_secs(60))),
            }),
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
                base: TakBaseGameSettings {
                    board_size: 5,
                    half_komi,
                    reserve: TakReserve::new(2, 0),
                },
                time_settings: TakTimeSettings::Realtime(TakRealtimeTimeControl {
                    contingent: Duration::from_secs(300),
                    increment: Duration::from_secs(5),
                    extra: Some((2, Duration::from_secs(60))),
                }),
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
                base: TakBaseGameSettings {
                    board_size: 5,
                    half_komi,
                    reserve: TakReserve::new(1, 1),
                },
                time_settings: TakTimeSettings::Realtime(TakRealtimeTimeControl {
                    contingent: Duration::from_secs(300),
                    increment: Duration::from_secs(5),
                    extra: Some((2, Duration::from_secs(60))),
                }),
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
            base: TakBaseGameSettings {
                board_size: 5,
                half_komi: 0,
                reserve: TakReserve::new(21, 1),
            },
            time_settings: TakTimeSettings::Realtime(TakRealtimeTimeControl {
                contingent: Duration::from_secs(300),
                increment: Duration::from_secs(5),
                extra: Some((2, Duration::from_secs(60))),
            }),
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
