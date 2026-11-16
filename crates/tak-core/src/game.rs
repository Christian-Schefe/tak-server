use std::time::{Duration, Instant, SystemTime};

use crate::{
    InvalidActionReason, InvalidUndoReason, TakAction, TakActionRecord, TakGameResult,
    TakGameSettings, TakPlayer, TakTimeInfo, TakTimeSettings, TakWinReason, base::TakBaseGame,
};

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

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MaybeTimeout<T> {
    Timeout(TakGameResult),
    NoTimeout(T),
}

#[derive(Clone, Debug)]
pub struct TakClock<T: TakInstant> {
    pub remaining_time: TakTimeInfo,
    pub is_ticking: Option<T>,
}

impl<T: TakInstant> TakClock<T> {
    fn end_turn(&mut self, settings: &TakTimeSettings, game: &TakBaseGame, player: TakPlayer) {
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
pub struct TakGame<T: TakInstant> {
    base: TakBaseGame,
    pub clock: TakClock<T>,
    pub time_settings: TakTimeSettings,
    pub history: Vec<TakActionRecord>,
}

impl<T: TakInstant> TakGame<T> {
    pub fn new(settings: &TakGameSettings) -> Self {
        let base_game = TakBaseGame::new(&settings.base);
        let clock = TakClock {
            remaining_time: settings.time_settings.initial_time_info(),
            is_ticking: None,
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
        let mut base = TakBaseGame::new(&settings.base);
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

    pub fn check_timeout(&mut self, now: T) -> MaybeTimeout<()> {
        let player = self.base.current_player;
        let time_remaining = self.get_time_remaining(player, now);
        if time_remaining.is_zero() {
            self.stop_clock(now);
            return MaybeTimeout::Timeout(TakGameResult::Win {
                winner: player.opponent(),
                reason: TakWinReason::Default,
            });
        }
        MaybeTimeout::NoTimeout(())
    }

    fn get_time_remaining(&self, player: TakPlayer, now: T) -> Duration {
        let base_remaining = match player {
            TakPlayer::White => self.clock.remaining_time.white_remaining,
            TakPlayer::Black => self.clock.remaining_time.black_remaining,
        };
        if let Some(last_update_timestamp) = self.clock.is_ticking
            && self.base.current_player == player
        {
            let elapsed = now.saturating_duration_since(last_update_timestamp);
            base_remaining.saturating_sub(elapsed)
        } else {
            base_remaining
        }
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
        if let Some(last_update_timestamp) = self.clock.is_ticking {
            let elapsed = now.saturating_duration_since(last_update_timestamp);
            *remaining = remaining.saturating_sub(elapsed);
        }

        self.clock.is_ticking = Some(now);
    }

    fn start_or_update_clock(&mut self, now: T, player: TakPlayer) {
        self.maybe_apply_elapsed(now, player);
        self.clock.end_turn(&self.time_settings, &self.base, player);
        self.clock.is_ticking = Some(now);
    }

    fn stop_clock(&mut self, now: T) {
        let player = self.base.current_player;
        self.maybe_apply_elapsed(now, player);
        self.clock.is_ticking = None;
    }

    pub fn do_action(
        &mut self,
        action: TakAction,
        now: T,
    ) -> MaybeTimeout<Result<Option<TakGameResult>, InvalidActionReason>> {
        if let MaybeTimeout::Timeout(result) = self.check_timeout(now) {
            return MaybeTimeout::Timeout(result);
        }

        let player = self.base.current_player;
        let remaining_before_action = self.get_time_info(now);
        let record = TakActionRecord {
            action: action.clone(),
            time_info: remaining_before_action,
        };
        match self.base.do_action(action) {
            Err(e) => return MaybeTimeout::NoTimeout(Err(e)),
            Ok(None) => {
                self.history.push(record);
                self.start_or_update_clock(now, player);
                MaybeTimeout::NoTimeout(Ok(None))
            }
            Ok(Some(result)) => {
                self.history.push(record);
                self.stop_clock(now);
                MaybeTimeout::NoTimeout(Ok(Some(result)))
            }
        }
    }

    pub fn try_undo_action(&mut self, now: T) -> MaybeTimeout<Result<(), InvalidUndoReason>> {
        if let MaybeTimeout::Timeout(result) = self.check_timeout(now) {
            return MaybeTimeout::Timeout(result);
        }
        let Some(record) = self.history.pop() else {
            return MaybeTimeout::NoTimeout(Err(InvalidUndoReason::NoHistory));
        };

        let mut new_base = TakBaseGame::new(&self.base.settings);
        for record in &self.history {
            match new_base.do_action(record.action.clone()) {
                Ok(None) => {}
                Ok(Some(_)) => {
                    return MaybeTimeout::NoTimeout(Err(
                        InvalidUndoReason::FailedToReconstructGame,
                    ));
                }
                Err(_) => {
                    return MaybeTimeout::NoTimeout(Err(
                        InvalidUndoReason::FailedToReconstructGame,
                    ));
                }
            }
        }
        self.clock.remaining_time = record.time_info;
        self.clock.is_ticking = Some(now);
        self.base = new_base;
        MaybeTimeout::NoTimeout(Ok(()))
    }

    pub fn resign_or_abandon(&mut self, player: TakPlayer, now: T) -> MaybeTimeout<TakGameResult> {
        if let MaybeTimeout::Timeout(result) = self.check_timeout(now) {
            return MaybeTimeout::Timeout(result);
        }
        self.stop_clock(now);
        MaybeTimeout::NoTimeout(TakGameResult::Win {
            winner: player.opponent(),
            reason: TakWinReason::Default,
        })
    }

    pub fn try_abort(&mut self, now: T) -> MaybeTimeout<Option<TakGameResult>> {
        if let MaybeTimeout::Timeout(result) = self.check_timeout(now) {
            return MaybeTimeout::Timeout(result);
        }
        if self.base.ply_index <= 1 {
            // if one player hasn't made a move yet, both players can abort without penalty
            self.stop_clock(now);
            return MaybeTimeout::NoTimeout(Some(TakGameResult::Aborted));
        }
        MaybeTimeout::NoTimeout(None)
    }

    pub fn abort_forced(&mut self, now: T) -> MaybeTimeout<TakGameResult> {
        if let MaybeTimeout::Timeout(result) = self.check_timeout(now) {
            return MaybeTimeout::Timeout(result);
        }
        self.stop_clock(now);
        MaybeTimeout::NoTimeout(TakGameResult::Aborted)
    }

    pub fn agree_draw(&mut self, now: T) -> MaybeTimeout<TakGameResult> {
        if let MaybeTimeout::Timeout(result) = self.check_timeout(now) {
            return MaybeTimeout::Timeout(result);
        }
        self.stop_clock(now);
        MaybeTimeout::NoTimeout(TakGameResult::Draw)
    }

    pub fn give_time_to_player(
        &mut self,
        player: TakPlayer,
        duration: Duration,
        now: T,
    ) -> MaybeTimeout<()> {
        if let MaybeTimeout::Timeout(result) = self.check_timeout(now) {
            return MaybeTimeout::Timeout(result);
        }

        let remaining = match player {
            TakPlayer::White => &mut self.clock.remaining_time.white_remaining,
            TakPlayer::Black => &mut self.clock.remaining_time.black_remaining,
        };
        *remaining = remaining.saturating_add(duration);

        MaybeTimeout::NoTimeout(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        InvalidPlaceReason, TakBaseGameSettings, TakDir, TakOpening, TakPos,
        TakRealtimeTimeControl, TakReserve, TakVariant,
    };

    use super::*;

    fn do_move<T: TakInstant>(game: &mut TakGame<T>, action: TakAction, now: T) {
        match game.do_action(action, now) {
            MaybeTimeout::NoTimeout(Ok(None)) => {}
            MaybeTimeout::NoTimeout(Ok(Some(result))) => {
                panic!(
                    "Game should not have finished, but finished with result: {:?}",
                    result
                )
            }
            MaybeTimeout::NoTimeout(Err(e)) => panic!("Invalid action: {:?}", e),
            MaybeTimeout::Timeout(e) => panic!("Action timed out: {:?}", e),
        }
    }

    fn do_finish_move<T: TakInstant>(
        game: &mut TakGame<T>,
        action: TakAction,
        now: T,
        expected_result: TakGameResult,
    ) {
        match game.do_action(action, now) {
            MaybeTimeout::NoTimeout(Ok(None)) => {
                panic!(
                    "Game should have finished with result: {:?}, but did not finish",
                    expected_result
                );
            }
            MaybeTimeout::NoTimeout(Ok(Some(result))) => {
                if result != expected_result {
                    panic!(
                        "Game should have finished with result: {:?}, but finished with result: {:?}",
                        expected_result, result
                    );
                }
            }
            MaybeTimeout::NoTimeout(Err(e)) => panic!("Invalid action: {:?}", e),
            MaybeTimeout::Timeout(e) => panic!("Action timed out: {:?}", e),
        }
    }

    #[test]
    fn test_reserve_constraints() {
        let now = Instant::now();
        let mut game = TakGame::new(&TakGameSettings {
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
            ) == MaybeTimeout::NoTimeout(Err(InvalidActionReason::InvalidPlace(
                InvalidPlaceReason::NoPiecesRemaining,
            )))
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
            ) == MaybeTimeout::NoTimeout(Err(InvalidActionReason::InvalidPlace(
                InvalidPlaceReason::NoPiecesRemaining,
            )))
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
            let mut game = TakGame::new(&TakGameSettings {
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

            let mut game2 = TakGame::new(&TakGameSettings {
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
        let mut game = TakGame::new(&TakGameSettings {
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
        });

        // first move must be flat stone
        assert!(
            game.do_action(
                TakAction::Place {
                    pos: TakPos::new(0, 0),
                    variant: TakVariant::Capstone,
                },
                now
            ) == MaybeTimeout::NoTimeout(Err(InvalidActionReason::InvalidPlace(
                InvalidPlaceReason::NoPiecesRemaining,
            )))
        );
        assert!(
            game.do_action(
                TakAction::Place {
                    pos: TakPos::new(0, 0),
                    variant: TakVariant::Standing,
                },
                now
            ) == MaybeTimeout::NoTimeout(Err(InvalidActionReason::InvalidPlace(
                InvalidPlaceReason::NoPiecesRemaining,
            )))
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
            ) == MaybeTimeout::NoTimeout(Err(InvalidActionReason::InvalidPlace(
                InvalidPlaceReason::NoPiecesRemaining,
            )))
        );
        assert!(
            game.do_action(
                TakAction::Place {
                    pos: TakPos::new(1, 0),
                    variant: TakVariant::Standing,
                },
                now
            ) == MaybeTimeout::NoTimeout(Err(InvalidActionReason::InvalidPlace(
                InvalidPlaceReason::NoPiecesRemaining,
            )))
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
            ) == MaybeTimeout::NoTimeout(Err(InvalidActionReason::OpeningViolation))
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
