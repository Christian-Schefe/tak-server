use std::time::{Duration, SystemTime};

use tak_core::{
    InvalidUndoReason, MaybeTimeout, TakAction, TakActionRecord, TakClock, TakGame, TakGameResult,
    TakGameSettings, TakPlayer, TakTimeInfo, TakTimeSettings,
};

#[derive(Debug, Clone)]
pub enum GameEventType {
    ActionPerformed {
        player: TakPlayer,
        ply_index: usize,
        action: TakAction,
    },
    ActionUndone {
        ply_index: usize,
    },
    GameOver {
        result: TakGameResult,
    },
}

#[derive(Debug, Clone)]
pub struct GameEvent {
    pub event_type: GameEventType,
    pub time_info: TakTimeInfo,
}

#[derive(Debug, Clone)]
pub struct GameState {
    pub history: Vec<TakActionRecord>,
    pub clock: TakClock<SystemTime>,
    pub white_requests: GameRequests,
    pub black_requests: GameRequests,
}

#[derive(Debug, Clone)]
pub struct GameFinished {
    pub history: Vec<TakActionRecord>,
    pub result: TakGameResult,
    pub time_info: TakTimeInfo,
}

#[derive(Debug, Clone)]
pub struct Game {
    pub game: TakGame<SystemTime>,
    pub white_requests: GameRequests,
    pub black_requests: GameRequests,
    pub event_log: Vec<GameEvent>,
}

#[derive(Debug, Clone)]
pub enum GameDoActionError {
    Timeout,
    NotPlayersTurn,
    InvalidAction(tak_core::InvalidActionReason),
}

pub enum GameResignError {
    Timeout,
}

#[derive(Debug, Clone)]
pub enum GameUndoActionError {
    Timeout,
    NoUndoAvailable,
    FailedToUndo(InvalidUndoReason),
}

#[derive(Debug, Clone)]
pub enum GameAbortError {
    Timeout,
    CantAbort,
}

#[derive(Debug, Clone)]
pub enum GameSetRequestError {
    Timeout,
    RequestAlreadySet,
}

#[derive(Debug, Clone)]
pub enum GameAcceptRequestError {
    Timeout,
    NoRequestToAccept,
}

#[derive(Debug, Clone)]
pub enum GameCheckTimeoutError {
    Timeout,
}

#[derive(Debug, Clone)]
pub enum CheckDisconnectTimeoutResult {
    TimedOut,
    CantTimeOut,
    NoTimeout(Duration),
}

#[derive(Debug, Clone)]
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
            event_log: Vec::new(),
        }
    }

    pub fn from_state(
        settings: &TakGameSettings,
        game_state: GameState,
    ) -> Result<Self, GameCreationError> {
        let game = TakGame::from_history(settings, game_state.clock, game_state.history)
            .map_err(|_| GameCreationError::InvalidState)?;
        Ok(Self {
            game,
            white_requests: game_state.white_requests,
            black_requests: game_state.black_requests,
            event_log: Vec::new(),
        })
    }

    pub fn drain_event_log(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.event_log)
    }

    pub fn to_game_state(self) -> GameState {
        GameState {
            history: self.game.history,
            clock: self.game.clock,
            white_requests: self.white_requests,
            black_requests: self.black_requests,
        }
    }

    fn handle_game_over(&mut self, result: TakGameResult) {
        self.event_log.push(GameEvent {
            event_type: GameEventType::GameOver { result },
            time_info: self.game.clock.remaining_time,
        });
    }

    pub fn do_action(
        &mut self,
        player: TakPlayer,
        action: TakAction,
        now: SystemTime,
    ) -> Result<(), GameDoActionError> {
        if self.game.current_player() != player {
            return Err(GameDoActionError::NotPlayersTurn);
        }
        match self.game.do_action(action.clone(), now) {
            MaybeTimeout::Timeout(result) => {
                self.handle_game_over(result);
                Err(GameDoActionError::Timeout)
            }
            MaybeTimeout::NoTimeout(Err(e)) => Err(GameDoActionError::InvalidAction(e)),
            MaybeTimeout::NoTimeout(Ok(res)) => {
                let ply_index = self.game.ply_index();
                self.event_log.push(GameEvent {
                    event_type: GameEventType::ActionPerformed {
                        action: action.clone(),
                        player,
                        ply_index,
                    },
                    time_info: self.game.get_time_info(now),
                });
                if let Some(result) = res {
                    self.handle_game_over(result);
                }
                Ok(())
            }
        }
    }

    pub fn resign(&mut self, player: TakPlayer, now: SystemTime) -> Result<(), GameResignError> {
        match self.game.resign_or_abandon(player, now) {
            MaybeTimeout::Timeout(result) => {
                self.handle_game_over(result);
                Err(GameResignError::Timeout)
            }
            MaybeTimeout::NoTimeout(result) => {
                self.handle_game_over(result);
                Ok(())
            }
        }
    }

    pub fn abort(&mut self, now: SystemTime) -> Result<(), GameAbortError> {
        match self.game.try_abort(now) {
            MaybeTimeout::Timeout(result) => {
                self.handle_game_over(result);
                Err(GameAbortError::Timeout)
            }
            MaybeTimeout::NoTimeout(None) => Err(GameAbortError::CantAbort),
            MaybeTimeout::NoTimeout(Some(result)) => {
                self.handle_game_over(result);
                Ok(())
            }
        }
    }

    pub fn abort_forced(&mut self, now: SystemTime) {
        match self.game.abort_forced(now) {
            MaybeTimeout::Timeout(result) => {
                self.handle_game_over(result);
            }
            MaybeTimeout::NoTimeout(result) => {
                self.handle_game_over(result);
            }
        }
    }

    pub fn set_request(
        &mut self,
        player: TakPlayer,
        request: GameRequest,
        now: SystemTime,
    ) -> Result<(), GameSetRequestError> {
        if let MaybeTimeout::Timeout(result) = self.game.check_timeout(now) {
            self.handle_game_over(result);
            return Err(GameSetRequestError::Timeout);
        }
        let requests = match player {
            TakPlayer::White => &mut self.white_requests,
            TakPlayer::Black => &mut self.black_requests,
        };
        if !requests.set_request(&request) {
            return Err(GameSetRequestError::RequestAlreadySet);
        }
        Ok(())
    }

    pub fn accept_draw_request(
        &mut self,
        player: TakPlayer,
        now: SystemTime,
    ) -> Result<(), GameAcceptRequestError> {
        if let MaybeTimeout::Timeout(result) = self.game.check_timeout(now) {
            self.handle_game_over(result);
            return Err(GameAcceptRequestError::Timeout);
        }
        let requests = match player.opponent() {
            TakPlayer::White => &mut self.white_requests,
            TakPlayer::Black => &mut self.black_requests,
        };
        if !requests.accept_draw_request() {
            return Err(GameAcceptRequestError::NoRequestToAccept);
        };
        match self.game.agree_draw(now) {
            MaybeTimeout::Timeout(result) => {
                self.handle_game_over(result);
                Err(GameAcceptRequestError::Timeout)
            }
            MaybeTimeout::NoTimeout(result) => {
                self.handle_game_over(result);
                Ok(())
            }
        }
    }

    pub fn accept_undo_request(
        &mut self,
        player: TakPlayer,
        now: SystemTime,
    ) -> Result<(), GameUndoActionError> {
        if let MaybeTimeout::Timeout(result) = self.game.check_timeout(now) {
            self.handle_game_over(result);
            return Err(GameUndoActionError::Timeout);
        }

        let requests = match player.opponent() {
            TakPlayer::White => &mut self.white_requests,
            TakPlayer::Black => &mut self.black_requests,
        };
        if !requests.accept_undo_request() {
            return Err(GameUndoActionError::NoUndoAvailable);
        };
        match self.game.try_undo_action(now) {
            MaybeTimeout::Timeout(result) => {
                self.handle_game_over(result);
                Err(GameUndoActionError::Timeout)
            }
            MaybeTimeout::NoTimeout(Err(e)) => Err(GameUndoActionError::FailedToUndo(e)),
            MaybeTimeout::NoTimeout(Ok(())) => {
                let ply_index = self.game.ply_index();
                self.event_log.push(GameEvent {
                    event_type: GameEventType::ActionUndone { ply_index },
                    time_info: self.game.get_time_info(now),
                });
                Ok(())
            }
        }
    }
    pub fn accept_more_time_request(
        &mut self,
        player: TakPlayer,
        now: SystemTime,
    ) -> Result<(), GameAcceptRequestError> {
        if let MaybeTimeout::Timeout(result) = self.game.check_timeout(now) {
            self.handle_game_over(result);
            return Err(GameAcceptRequestError::Timeout);
        }

        let requests = match player.opponent() {
            TakPlayer::White => &mut self.white_requests,
            TakPlayer::Black => &mut self.black_requests,
        };
        let Some(duration) = requests.accept_more_time_request() else {
            return Err(GameAcceptRequestError::NoRequestToAccept);
        };
        match self.game.give_time_to_player(player, duration, now) {
            MaybeTimeout::Timeout(result) => {
                self.handle_game_over(result);
                return Err(GameAcceptRequestError::Timeout);
            }
            MaybeTimeout::NoTimeout(()) => Ok(()),
        }
    }
    pub fn check_timeout(&mut self, now: SystemTime) -> Result<TakTimeInfo, GameCheckTimeoutError> {
        if let MaybeTimeout::Timeout(result) = self.game.check_timeout(now) {
            self.handle_game_over(result);
            return Err(GameCheckTimeoutError::Timeout);
        }

        let time_info = self.game.get_time_info(now);
        Ok(time_info)
    }
    pub fn check_disconnect_timeout(
        &mut self,
        player: TakPlayer,
        disconnected_duration: Duration,
        now: SystemTime,
    ) -> Result<CheckDisconnectTimeoutResult, GameCheckTimeoutError> {
        if let MaybeTimeout::Timeout(result) = self.game.check_timeout(now) {
            self.handle_game_over(result);
            return Err(GameCheckTimeoutError::Timeout);
        }
        if matches!(self.game.time_settings, TakTimeSettings::Async(_)) {
            return Ok(CheckDisconnectTimeoutResult::CantTimeOut);
        }
        let timeout_duration = Duration::from_secs(60 * 5);
        if disconnected_duration < timeout_duration {
            return Ok(CheckDisconnectTimeoutResult::NoTimeout(
                timeout_duration - disconnected_duration,
            ));
        }
        match self.game.resign_or_abandon(player, now) {
            MaybeTimeout::Timeout(result) => {
                self.handle_game_over(result);
                Err(GameCheckTimeoutError::Timeout)
            }
            MaybeTimeout::NoTimeout(result) => {
                self.handle_game_over(result);
                Ok(CheckDisconnectTimeoutResult::TimedOut)
            }
        }
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
