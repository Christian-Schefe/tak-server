use std::time::{Duration, Instant};

use crate::{
    InvalidActionReason, MaybeTimeout, TakAction, TakAsyncTimeControl, TakGameResult,
    TakGameSettings, TakInstant, TakPlayer, TakRealtimeTimeControl, TakTimeInfo, TakTimeSettings,
    TakWinReason,
    base::{TakFinishedBaseGame, TakOngoingBaseGame},
};

#[derive(Clone, Debug)]
pub struct TakFinishedGame {
    base: TakFinishedBaseGame,
    time_info: TakTimeInfo,
}

impl TakFinishedGame {
    fn new(ongoing_game: &TakOngoingGame, game_result: TakGameResult, now: TakInstant) -> Self {
        TakFinishedGame {
            base: TakFinishedBaseGame::new(&ongoing_game.base, game_result),
            time_info: ongoing_game.get_time_info(now),
        }
    }

    fn from_finished_base(
        finished_base: TakFinishedBaseGame,
        ongoing_game: &TakOngoingGame,
        now: TakInstant,
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
pub struct TakOngoingGame {
    mode: TakGameMode,
    base: TakOngoingBaseGame,
}

#[derive(Clone, Debug)]
enum TakGameMode {
    Realtime {
        time_settings: TakRealtimeTimeControl,
        clock: TakRealtimeClock,
    },
    Async {
        time_settings: TakAsyncTimeControl,
        clock: TakAsyncClock,
    },
}

#[derive(Clone, Debug)]
struct TakRealtimeClock {
    remaining_time: (Duration, Duration),
    last_update_timestamp: Instant,
    has_gained_extra_time: (bool, bool),
    is_ticking: bool,
}
#[derive(Clone, Debug)]
struct TakAsyncClock {
    deadline: chrono::DateTime<chrono::Utc>,
    is_ticking: bool,
}

impl TakOngoingGame {
    pub fn new(settings: TakGameSettings) -> Self {
        let base_game = TakOngoingBaseGame::new(settings.base);
        let mode = match settings.time_settings {
            TakTimeSettings::Realtime(settings) => TakGameMode::Realtime {
                clock: TakRealtimeClock {
                    remaining_time: (settings.contingent, settings.contingent),
                    last_update_timestamp: Instant::now(),
                    has_gained_extra_time: (false, false),
                    is_ticking: false,
                },
                time_settings: settings,
            },
            TakTimeSettings::Async(settings) => TakGameMode::Async {
                clock: TakAsyncClock {
                    deadline: chrono::Utc::now(),
                    is_ticking: false,
                },
                time_settings: settings,
            },
        };
        TakOngoingGame {
            base: base_game,
            mode,
        }
    }

    pub fn action_history(&self) -> &Vec<TakAction> {
        &self.base.action_history
    }

    pub fn current_player(&self) -> TakPlayer {
        self.base.current_player
    }

    fn set_game_over(&mut self, now: TakInstant, game_result: TakGameResult) -> TakFinishedGame {
        let player = self.base.current_player.clone();
        self.stop_clock(now, &player);
        TakFinishedGame::new(self, game_result, now)
    }

    pub fn check_timeout(&mut self, now: TakInstant) -> MaybeTimeout<(), TakFinishedGame> {
        match &self.mode {
            TakGameMode::Realtime { clock, .. } => {
                let player = self.base.current_player.clone();
                let time_remaining = Self::get_time_remaining(clock, &self.base, &player, now);
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
            TakGameMode::Async { clock, .. } => {
                let player = self.base.current_player.clone();
                if clock.deadline <= now.async_time {
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
        }
    }

    fn get_time_remaining(
        clock: &TakRealtimeClock,
        base: &TakOngoingBaseGame,
        player: &TakPlayer,
        now: TakInstant,
    ) -> Duration {
        let base_remaining = match player {
            TakPlayer::White => clock.remaining_time.0,
            TakPlayer::Black => clock.remaining_time.1,
        };
        if base.current_player != *player || !clock.is_ticking {
            return base_remaining;
        }
        let elapsed = now
            .realtime
            .saturating_duration_since(clock.last_update_timestamp);
        base_remaining.saturating_sub(elapsed)
    }

    pub fn get_time_info(&self, now: TakInstant) -> TakTimeInfo {
        match &self.mode {
            TakGameMode::Realtime { clock, .. } => TakTimeInfo::Realtime {
                white_remaining: Self::get_time_remaining(
                    clock,
                    &self.base,
                    &TakPlayer::White,
                    now,
                ),
                black_remaining: Self::get_time_remaining(
                    clock,
                    &self.base,
                    &TakPlayer::Black,
                    now,
                ),
            },
            TakGameMode::Async { clock, .. } => TakTimeInfo::Async {
                next_deadline: clock.deadline,
            },
        }
    }

    fn increase_deadline(
        clock: &mut TakAsyncClock,
        settings: &TakAsyncTimeControl,
        now: chrono::DateTime<chrono::Utc>,
    ) {
        clock.deadline = now + settings.increment;
    }

    fn maybe_apply_elapsed(
        clock: &mut TakRealtimeClock,
        settings: &TakRealtimeTimeControl,
        now: Instant,
        player: &TakPlayer,
        add_increment: bool,
    ) {
        let remaining = match player {
            TakPlayer::White => &mut clock.remaining_time.0,
            TakPlayer::Black => &mut clock.remaining_time.1,
        };
        if clock.is_ticking {
            let elapsed = now.saturating_duration_since(clock.last_update_timestamp);
            *remaining = remaining.saturating_sub(elapsed);
        }
        if add_increment {
            *remaining = remaining.saturating_add(settings.increment);
        }
        clock.last_update_timestamp = now;
    }

    fn maybe_gain_extra_time(
        clock: &mut TakRealtimeClock,
        settings: &TakRealtimeTimeControl,
        base: &TakOngoingBaseGame,
        player: &TakPlayer,
    ) {
        if !clock.is_ticking {
            return;
        }
        let remaining = match player {
            TakPlayer::White => &mut clock.remaining_time.0,
            TakPlayer::Black => &mut clock.remaining_time.1,
        };
        if let Some((extra_move_index, extra_time)) = settings.extra {
            let has_gained_extra_time = match player {
                TakPlayer::White => &mut clock.has_gained_extra_time.0,
                TakPlayer::Black => &mut clock.has_gained_extra_time.1,
            };
            // ply index is incremented before clock update, which means it is odd for white moves and starts at 1 for move 1
            // move 1: white 1, black 2 ---(+1)--> (2, 3) ---(/2)--> (1, 1)
            // move 2: white 3, black 4 ---(+1)--> (4, 5) ---(/2)--> (2, 2)
            let move_index = (base.action_history.len() + 1) / 2;
            if !*has_gained_extra_time && extra_move_index as usize == move_index {
                *remaining = remaining.saturating_add(extra_time);
                *has_gained_extra_time = true;
            }
        }
    }

    fn start_or_update_clock(&mut self, now: TakInstant, player: &TakPlayer) {
        match &mut self.mode {
            TakGameMode::Realtime {
                clock,
                time_settings,
            } => {
                Self::maybe_apply_elapsed(clock, time_settings, now.realtime, player, true);
                Self::maybe_gain_extra_time(clock, time_settings, &self.base, player);
                clock.is_ticking = true;
            }
            TakGameMode::Async {
                clock,
                time_settings,
            } => {
                Self::increase_deadline(clock, time_settings, now.async_time);
                clock.is_ticking = true;
            }
        }
    }

    fn stop_clock(&mut self, now: TakInstant, player: &TakPlayer) {
        match &mut self.mode {
            TakGameMode::Realtime {
                clock,
                time_settings,
            } => {
                Self::maybe_apply_elapsed(clock, time_settings, now.realtime, player, false);
                clock.is_ticking = false;
            }
            TakGameMode::Async { clock, .. } => {
                clock.is_ticking = false;
            }
        }
    }

    pub fn do_action(
        &mut self,
        action: TakAction,
        now: TakInstant,
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

    pub fn undo_action(&mut self, now: TakInstant) -> MaybeTimeout<bool, TakFinishedGame> {
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
        match &mut self.mode {
            TakGameMode::Realtime {
                clock,
                time_settings,
            } => {
                Self::maybe_apply_elapsed(clock, time_settings, now.realtime, &player, true); // TODO: confirm that undo is supposed to add increment
            }
            TakGameMode::Async {
                clock,
                time_settings,
            } => {
                Self::increase_deadline(clock, time_settings, now.async_time);
            }
        }
        MaybeTimeout::Result(true)
    }

    pub fn resign(
        &mut self,
        player: &TakPlayer,
        now: TakInstant,
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

    pub fn agree_draw(
        &mut self,
        now: TakInstant,
    ) -> MaybeTimeout<TakFinishedGame, TakFinishedGame> {
        if let MaybeTimeout::Timeout(finished_game) = self.check_timeout(now) {
            return MaybeTimeout::Timeout(finished_game);
        };
        MaybeTimeout::Result(self.set_game_over(now, TakGameResult::Draw))
    }

    pub fn give_time_to_player(
        &mut self,
        player: &TakPlayer,
        duration: Duration,
        now: TakInstant,
    ) -> MaybeTimeout<(), TakFinishedGame> {
        if let MaybeTimeout::Timeout(finished_game) = self.check_timeout(now) {
            return MaybeTimeout::Timeout(finished_game);
        };
        match &mut self.mode {
            TakGameMode::Realtime { clock, .. } => {
                let remaining = match player {
                    TakPlayer::White => &mut clock.remaining_time.0,
                    TakPlayer::Black => &mut clock.remaining_time.1,
                };
                *remaining = remaining.saturating_add(duration);
            }
            TakGameMode::Async { .. } => {}
        }

        MaybeTimeout::Result(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        TakBaseGameSettings, TakDir, TakPos, TakRealtimeTimeControl, TakReserve, TakVariant,
    };

    use super::*;

    fn do_move(game: &mut TakOngoingGame, action: TakAction, now: TakInstant) {
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
        now: TakInstant,
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
        let now = TakInstant::now();
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
        let now = TakInstant::now();
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
        let now = TakInstant::now();
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
