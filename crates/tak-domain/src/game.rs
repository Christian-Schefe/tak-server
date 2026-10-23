use std::time::{Duration, SystemTime};

use crate::game::request::{GameRequest, GameRequests};
use tak_core::{TakAction, TakGame, TakGameSettings, TakPlayer, TakTimeInfo, TakTimeSettings};

pub mod request;

#[derive(Clone, Debug)]
pub struct Game {
    pub game: TakGame<SystemTime>,
    pub white_requests: GameRequests,
    pub black_requests: GameRequests,
}

#[derive(Clone, Debug)]
pub struct GameActionRecord {
    pub action: TakAction,
    pub ply_index: usize,
    pub time_info: TakTimeInfo,
}

impl GameActionRecord {
    pub fn new(action: TakAction, ply_index: usize, time_info: TakTimeInfo) -> Self {
        Self {
            action,
            ply_index,
            time_info,
        }
    }
}

#[derive(Clone, Debug)]
pub struct GameUndoActionRecord {
    pub ply_index: usize,
}

impl GameUndoActionRecord {
    pub fn new(ply_index: usize) -> Self {
        Self { ply_index }
    }
}

pub enum DoActionResult {
    GameOver,
    ActionPerformed(GameActionRecord),
    NotPlayersTurn,
    InvalidAction(tak_core::InvalidActionReason),
}

pub enum GameOverResult<R> {
    GameOver,
    Result(R),
}

pub enum CheckDisconnectTimeoutResult {
    TimedOut,
    CantTimeOut,
    NoTimeout(Duration),
}

impl Game {
    pub fn new(settings: &TakGameSettings, now: SystemTime) -> Self {
        Self {
            game: TakGame::new(settings, now),
            white_requests: GameRequests::new(),
            black_requests: GameRequests::new(),
        }
    }

    pub fn do_action(
        &mut self,
        player: TakPlayer,
        action: TakAction,
        now: SystemTime,
    ) -> DoActionResult {
        if self.game.current_player() != player {
            return DoActionResult::NotPlayersTurn;
        }
        match self.game.do_action(action.clone(), now) {
            Err(e) => DoActionResult::InvalidAction(e),
            Ok(()) => {
                let ply_index = self.game.ply_index();
                let time_info = self.game.get_time_info(now);
                let record = GameActionRecord::new(action, ply_index, time_info);
                DoActionResult::ActionPerformed(record)
            }
        }
    }

    pub fn resign(&mut self, player: TakPlayer, now: SystemTime) -> GameOverResult<()> {
        match self.game.resign_or_abandon(player, now) {
            Err(()) => GameOverResult::GameOver,
            Ok(()) => GameOverResult::Result(()),
        }
    }

    pub fn try_abort(&mut self, now: SystemTime) -> GameOverResult<bool> {
        match self.game.try_abort(now) {
            Err(()) => GameOverResult::GameOver,
            Ok(did_abort) => GameOverResult::Result(did_abort),
        }
    }

    pub fn abort_forced(&mut self, now: SystemTime) -> GameOverResult<()> {
        match self.game.abort_forced(now) {
            Err(()) => GameOverResult::GameOver,
            Ok(()) => GameOverResult::Result(()),
        }
    }

    pub fn set_request(
        &mut self,
        player: TakPlayer,
        request: GameRequest,
        now: SystemTime,
    ) -> GameOverResult<bool> {
        if let Err(()) = self.game.validate_ongoing(now) {
            return GameOverResult::GameOver;
        }
        let requests = match player {
            TakPlayer::White => &mut self.white_requests,
            TakPlayer::Black => &mut self.black_requests,
        };
        GameOverResult::Result(requests.set_request(&request))
    }
    pub fn accept_draw_request(
        &mut self,
        player: TakPlayer,
        now: SystemTime,
    ) -> GameOverResult<bool> {
        if let Err(()) = self.game.validate_ongoing(now) {
            return GameOverResult::GameOver;
        }
        let requests = match player.opponent() {
            TakPlayer::White => &mut self.white_requests,
            TakPlayer::Black => &mut self.black_requests,
        };
        match requests.accept_draw_request() {
            true => {
                if let Err(()) = self.game.agree_draw(now) {
                    return GameOverResult::GameOver;
                }
                GameOverResult::Result(true)
            }
            false => GameOverResult::Result(false),
        }
    }
    pub fn accept_undo_request(
        &mut self,
        player: TakPlayer,
        now: SystemTime,
    ) -> GameOverResult<Option<Option<GameUndoActionRecord>>> {
        if let Err(()) = self.game.validate_ongoing(now) {
            return GameOverResult::GameOver;
        }

        let requests = match player.opponent() {
            TakPlayer::White => &mut self.white_requests,
            TakPlayer::Black => &mut self.black_requests,
        };
        if !requests.accept_undo_request() {
            return GameOverResult::Result(None);
        };
        match self.game.try_undo_action(now) {
            Err(_) => return GameOverResult::GameOver,
            Ok(false) => return GameOverResult::Result(Some(None)),
            Ok(true) => {}
        }

        let ply_index = self.game.ply_index();
        let undo_record = GameUndoActionRecord::new(ply_index);

        GameOverResult::Result(Some(Some(undo_record)))
    }
    pub fn accept_more_time_request(
        &mut self,
        player: TakPlayer,
        now: SystemTime,
    ) -> GameOverResult<bool> {
        if let Err(()) = self.game.validate_ongoing(now) {
            return GameOverResult::GameOver;
        }

        let requests = match player.opponent() {
            TakPlayer::White => &mut self.white_requests,
            TakPlayer::Black => &mut self.black_requests,
        };
        let Some(duration) = requests.accept_more_time_request() else {
            return GameOverResult::Result(false);
        };
        if let Err(()) = self.game.give_time_to_player(player, duration, now) {
            return GameOverResult::GameOver;
        }
        GameOverResult::Result(true)
    }
    pub fn check_timeout(&mut self, now: SystemTime) -> GameOverResult<TakTimeInfo> {
        if let Err(()) = self.game.validate_ongoing(now) {
            return GameOverResult::GameOver;
        }

        let time_info = self.game.get_time_info(now);
        GameOverResult::Result(time_info)
    }
    pub fn check_disconnect_timeout(
        &mut self,
        player: TakPlayer,
        disconnected_duration: Duration,
        now: SystemTime,
    ) -> GameOverResult<CheckDisconnectTimeoutResult> {
        if let Err(()) = self.game.validate_ongoing(now) {
            return GameOverResult::GameOver;
        }
        if matches!(self.game.time_settings, TakTimeSettings::Async(_)) {
            return GameOverResult::Result(CheckDisconnectTimeoutResult::CantTimeOut);
        }
        let timeout_duration = Duration::from_secs(60 * 5);
        if disconnected_duration < timeout_duration {
            return GameOverResult::Result(CheckDisconnectTimeoutResult::NoTimeout(
                timeout_duration - disconnected_duration,
            ));
        }
        if let Err(()) = self.game.resign_or_abandon(player, now) {
            return GameOverResult::GameOver;
        }
        GameOverResult::Result(CheckDisconnectTimeoutResult::TimedOut)
    }
}
