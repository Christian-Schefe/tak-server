use crate::{
    InvalidActionReason, MaybeTimeout, TakAction, TakAsyncGameSettings, TakGameResult, TakPlayer,
    TakWinReason,
    base::{TakFinishedBaseGame, TakOngoingBaseGame},
};

#[derive(Clone, Debug)]
pub struct TakFinishedAsyncGame {
    base: TakFinishedBaseGame,
}

impl TakFinishedAsyncGame {
    fn new(ongoing_game: &TakOngoingAsyncGame, game_result: TakGameResult) -> Self {
        TakFinishedAsyncGame {
            base: TakFinishedBaseGame::new(&ongoing_game.base, game_result),
        }
    }

    fn from_finished_base(
        finished_base: TakFinishedBaseGame,
        _ongoing_game: &TakOngoingAsyncGame,
    ) -> Self {
        TakFinishedAsyncGame {
            base: finished_base,
        }
    }

    pub fn action_history(&self) -> &Vec<TakAction> {
        &self.base.action_history
    }

    pub fn game_result(&self) -> &TakGameResult {
        &self.base.game_result
    }
}

#[derive(Clone, Debug)]
pub struct TakOngoingAsyncGame {
    settings: TakAsyncGameSettings,
    base: TakOngoingBaseGame,
    clock: TakDeadlineClock,
}

#[derive(Clone, Debug)]
pub struct TakDeadlineClock {
    deadline: (chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>),
    is_ticking: bool,
}

impl TakOngoingAsyncGame {
    pub fn new(settings: TakAsyncGameSettings) -> Self {
        let base_game = TakOngoingBaseGame::new(settings.base.clone());
        TakOngoingAsyncGame {
            base: base_game,
            clock: TakDeadlineClock {
                deadline: (chrono::Utc::now(), chrono::Utc::now()),
                is_ticking: false,
            },
            settings,
        }
    }

    pub fn action_history(&self) -> &Vec<TakAction> {
        &self.base.action_history
    }

    pub fn current_player(&self) -> TakPlayer {
        self.base.current_player
    }

    fn set_game_over(&mut self, game_result: TakGameResult) -> TakFinishedAsyncGame {
        self.stop_clock();
        TakFinishedAsyncGame::new(self, game_result)
    }

    pub fn check_deadline(
        &mut self,
        now: chrono::DateTime<chrono::Utc>,
    ) -> MaybeTimeout<(), TakFinishedAsyncGame> {
        let player = self.base.current_player.clone();
        let deadline = match player {
            TakPlayer::White => &mut self.clock.deadline.0,
            TakPlayer::Black => &mut self.clock.deadline.1,
        };
        if *deadline <= now {
            let game_result = TakGameResult::Win {
                winner: player.opponent(),
                reason: TakWinReason::Default,
            };
            self.stop_clock();
            MaybeTimeout::Timeout(TakFinishedAsyncGame::new(self, game_result))
        } else {
            MaybeTimeout::Result(())
        }
    }

    fn increase_deadline(&mut self, now: chrono::DateTime<chrono::Utc>, player: &TakPlayer) {
        let deadline = match player {
            TakPlayer::White => &mut self.clock.deadline.0,
            TakPlayer::Black => &mut self.clock.deadline.1,
        };
        *deadline = now + self.settings.time_control.increment;
    }

    fn start_or_update_clock(&mut self, now: chrono::DateTime<chrono::Utc>, player: &TakPlayer) {
        self.increase_deadline(now, player);

        self.clock.is_ticking = true;
    }

    fn stop_clock(&mut self) {
        self.clock.is_ticking = false;
    }

    pub fn do_action(
        &mut self,
        action: TakAction,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<MaybeTimeout<Option<TakFinishedAsyncGame>, TakFinishedAsyncGame>, InvalidActionReason>
    {
        if let MaybeTimeout::Timeout(finished_game) = self.check_deadline(now) {
            return Ok(MaybeTimeout::Timeout(finished_game));
        };

        let player = self.base.current_player.clone();

        match self.base.do_action(action) {
            Ok(None) => {
                self.start_or_update_clock(now, &player);
                Ok(MaybeTimeout::Result(None))
            }
            Ok(Some(finished_base)) => {
                self.stop_clock();
                let finished_game = TakFinishedAsyncGame::from_finished_base(finished_base, self);
                Ok(MaybeTimeout::Result(Some(finished_game)))
            }
            Err(e) => Err(e),
        }
    }

    pub fn undo_action(
        &mut self,
        now: chrono::DateTime<chrono::Utc>,
    ) -> MaybeTimeout<bool, TakFinishedAsyncGame> {
        if let MaybeTimeout::Timeout(finished_game) = self.check_deadline(now) {
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
        self.increase_deadline(now, &player);
        MaybeTimeout::Result(true)
    }

    pub fn resign(
        &mut self,
        player: &TakPlayer,
        now: chrono::DateTime<chrono::Utc>,
    ) -> MaybeTimeout<TakFinishedAsyncGame, TakFinishedAsyncGame> {
        if let MaybeTimeout::Timeout(finished_game) = self.check_deadline(now) {
            return MaybeTimeout::Timeout(finished_game);
        };
        MaybeTimeout::Result(self.set_game_over(TakGameResult::Win {
            winner: player.opponent(),
            reason: TakWinReason::Default,
        }))
    }

    pub fn agree_draw(
        &mut self,
        now: chrono::DateTime<chrono::Utc>,
    ) -> MaybeTimeout<TakFinishedAsyncGame, TakFinishedAsyncGame> {
        if let MaybeTimeout::Timeout(finished_game) = self.check_deadline(now) {
            return MaybeTimeout::Timeout(finished_game);
        };
        MaybeTimeout::Result(self.set_game_over(TakGameResult::Draw))
    }
}
