use std::time::{Duration, SystemTime};

use tak_core::{
    TakAction, TakActionRecord, TakClock, TakGame, TakGameResult, TakGameSettings, TakPlayer,
    TakTimeInfo, TakTimeSettings,
};

#[derive(Debug, Clone)]
pub enum GameState {
    Ongoing(OngoingGameState),
    Finished(FinishedGameState),
}

#[derive(Debug, Clone)]
pub struct OngoingGameState {
    pub history: Vec<TakActionRecord>,
    pub clock: TakClock<SystemTime>,
    pub white_requests: GameRequests,
    pub black_requests: GameRequests,
}

#[derive(Debug, Clone)]
pub struct FinishedGameState {
    pub history: Vec<TakActionRecord>,
    pub time_info: TakTimeInfo,
    pub result: TakGameResult,
}

#[derive(Debug, Clone)]
pub struct Game {
    pub game: TakGame<SystemTime>,
    pub white_requests: GameRequests,
    pub black_requests: GameRequests,
}

pub enum DoActionResult {
    GameOver,
    ActionPerformed { action: TakAction, ply_index: usize },
    NotPlayersTurn,
    InvalidAction(tak_core::InvalidActionReason),
}

pub enum UndoActionResult {
    Success { ply_index: usize },
    NoUndoAvailable,
    NothingToUndo,
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

pub enum GameCreationError {
    InvalidState,
}

impl Game {
    pub fn new(settings: &TakGameSettings) -> Self {
        let game = TakGame::new(settings);
        Self {
            game,
            white_requests: GameRequests::new(),
            black_requests: GameRequests::new(),
        }
    }

    pub fn from_state(
        settings: &TakGameSettings,
        game_state: OngoingGameState,
    ) -> Result<Self, GameCreationError> {
        let game = TakGame::from_history(settings, game_state.clock, game_state.history)
            .map_err(|_| GameCreationError::InvalidState)?;
        Ok(Self {
            game,
            white_requests: game_state.white_requests,
            black_requests: game_state.black_requests,
        })
    }

    pub fn to_game_state(self) -> GameState {
        match self.game.result() {
            Some(result) => GameState::Finished(FinishedGameState {
                result: result.clone(),
                history: self.game.history,
                time_info: self.game.clock.remaining_time,
            }),
            None => GameState::Ongoing(OngoingGameState {
                history: self.game.history,
                clock: self.game.clock,
                white_requests: self.white_requests,
                black_requests: self.black_requests,
            }),
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
                DoActionResult::ActionPerformed { action, ply_index }
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
    ) -> GameOverResult<UndoActionResult> {
        if let Err(()) = self.game.validate_ongoing(now) {
            return GameOverResult::GameOver;
        }

        let requests = match player.opponent() {
            TakPlayer::White => &mut self.white_requests,
            TakPlayer::Black => &mut self.black_requests,
        };
        if !requests.accept_undo_request() {
            return GameOverResult::Result(UndoActionResult::NoUndoAvailable);
        };
        match self.game.try_undo_action(now) {
            Err(_) => return GameOverResult::GameOver,
            Ok(false) => return GameOverResult::Result(UndoActionResult::NothingToUndo),
            Ok(true) => {}
        }

        let ply_index = self.game.ply_index();

        GameOverResult::Result(UndoActionResult::Success { ply_index })
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

#[derive(Clone, Debug)]
pub enum GameRequest {
    Draw(bool),
    Undo(bool),
    MoreTime(Option<Duration>),
}

#[derive(Clone, Debug)]
pub enum GameRequestType {
    Draw,
    Undo,
    MoreTime,
}

#[derive(Clone, Debug)]
pub struct GameRequests {
    pub draw_offered: bool,
    pub undo_requested: bool,
    pub more_time_offered: Option<Duration>,
}

impl GameRequests {
    pub fn new() -> Self {
        GameRequests {
            draw_offered: false,
            undo_requested: false,
            more_time_offered: None,
        }
    }

    pub fn set_request(&mut self, request: &GameRequest) -> bool {
        match request {
            GameRequest::Draw(offer) => {
                if self.draw_offered == *offer {
                    return false;
                }
                self.draw_offered = *offer;
            }
            GameRequest::Undo(request) => {
                if self.undo_requested == *request {
                    return false;
                }
                self.undo_requested = *request;
            }
            GameRequest::MoreTime(duration) => {
                if self.more_time_offered == *duration {
                    return false;
                }
                self.more_time_offered = *duration;
            }
        }
        return true;
    }

    pub fn accept_draw_request(&mut self) -> bool {
        if !self.draw_offered {
            return false;
        }
        self.draw_offered = false;
        true
    }

    pub fn accept_undo_request(&mut self) -> bool {
        if !self.undo_requested {
            return false;
        }
        self.undo_requested = false;
        true
    }

    pub fn accept_more_time_request(&mut self) -> Option<Duration> {
        if self.more_time_offered.is_none() {
            return None;
        }
        let duration = self.more_time_offered.take();
        duration
    }
}
