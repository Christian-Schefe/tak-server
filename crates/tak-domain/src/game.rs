use std::time::{Duration, SystemTime};

use crate::{
    MatchId, UserId,
    game::request::{GameRequest, GameRequestType, GameRequests},
    game_record::GameRatingInfo,
};
use tak_core::{
    MaybeTimeout, TakAction, TakFinishedGame, TakGameSettings, TakOngoingGame, TakPlayer,
    TakTimeInfo, TakTimeSettings,
};

pub mod request;

#[derive(Debug, Clone)]
pub struct PlayerSnapshot {
    pub user_id: UserId,
    pub username: String,
    pub rating: Option<f64>,
}

impl PlayerSnapshot {
    pub fn new(user_id: UserId, username: String, rating: Option<f64>) -> Self {
        Self {
            user_id,
            username,
            rating,
        }
    }
}

#[derive(Clone, Debug)]
pub struct GameMetadata {
    pub date: chrono::DateTime<chrono::Utc>,
    pub white: PlayerSnapshot,
    pub black: PlayerSnapshot,
    pub settings: TakGameSettings,
    pub is_rated: bool,
    pub match_id: Option<MatchId>,
}

impl GameMetadata {
    pub fn get_opponent(&self, player: UserId) -> Option<UserId> {
        if player == self.white.user_id {
            Some(self.black.user_id)
        } else if player == self.black.user_id {
            Some(self.white.user_id)
        } else {
            None
        }
    }
    pub fn get_user_id(&self, player: TakPlayer) -> UserId {
        match player {
            TakPlayer::White => self.white.user_id,
            TakPlayer::Black => self.black.user_id,
        }
    }

    pub fn get_player(&self, id: UserId) -> Option<TakPlayer> {
        if id == self.white.user_id {
            Some(TakPlayer::White)
        } else if id == self.black.user_id {
            Some(TakPlayer::Black)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
pub struct Game {
    pub game: TakOngoingGame<SystemTime>,
    pub white_requests: GameRequests,
    pub black_requests: GameRequests,
}

#[derive(Clone, Debug)]
pub struct FinishedGameState {
    pub game: TakFinishedGame,
    pub rating_info: Option<GameRatingInfo>,
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
    ActionPerformed(GameActionRecord),
    GameOver(TakFinishedGame, GameActionRecord),
    NotPlayersTurn,
    InvalidAction(tak_core::InvalidActionReason),
}

pub enum GamePlayerActionResult<R> {
    Timeout(TakFinishedGame),
    Result(R),
}

pub enum CheckTimeoutResult {
    TimedOut(TakFinishedGame),
    NoTimeout(TakTimeInfo),
}

pub enum CheckDisconnectTimeoutResult {
    TimedOut(TakFinishedGame),
    CantTimeOut,
    NoTimeout(Duration),
}

impl GameMetadata {
    pub fn new(
        date: chrono::DateTime<chrono::Utc>,
        white: PlayerSnapshot,
        black: PlayerSnapshot,
        is_rated: bool,
        settings: TakGameSettings,
        match_id: Option<MatchId>,
    ) -> Self {
        Self {
            date,
            white,
            black,
            settings,
            is_rated,
            match_id,
        }
    }
}

impl Game {
    pub fn new(settings: &TakGameSettings, now: SystemTime) -> Self {
        Self {
            game: TakOngoingGame::new(settings, now),
            white_requests: GameRequests::new(),
            black_requests: GameRequests::new(),
        }
    }

    pub fn do_action(
        &mut self,
        player: TakPlayer,
        action: TakAction,
        now: SystemTime,
    ) -> GamePlayerActionResult<DoActionResult> {
        if self.game.current_player() != player {
            return GamePlayerActionResult::Result(DoActionResult::NotPlayersTurn);
        }
        match self.game.do_action(action.clone(), now) {
            Err(e) => GamePlayerActionResult::Result(DoActionResult::InvalidAction(e)),
            Ok(MaybeTimeout::Timeout(finished_game)) => {
                GamePlayerActionResult::Timeout(finished_game)
            }
            Ok(MaybeTimeout::Result(Some(finished_game))) => {
                let ply_index = self.game.ply_index();
                let time_info = finished_game.time_info.clone();
                GamePlayerActionResult::Result(DoActionResult::GameOver(
                    finished_game,
                    GameActionRecord::new(action, ply_index, time_info),
                ))
            }
            Ok(MaybeTimeout::Result(None)) => {
                let ply_index = self.game.ply_index();
                let time_info = self.game.get_time_info(now);
                GamePlayerActionResult::Result(DoActionResult::ActionPerformed(
                    GameActionRecord::new(action, ply_index, time_info),
                ))
            }
        }
    }
    pub fn resign(
        &mut self,
        player: TakPlayer,
        now: SystemTime,
    ) -> GamePlayerActionResult<TakFinishedGame> {
        match self.game.resign_or_abandon(player, now) {
            MaybeTimeout::Timeout(finished_game) => GamePlayerActionResult::Timeout(finished_game),
            MaybeTimeout::Result(finished_game) => GamePlayerActionResult::Result(finished_game),
        }
    }
    pub fn try_abort(
        &mut self,
        now: SystemTime,
    ) -> GamePlayerActionResult<Option<TakFinishedGame>> {
        match self.game.try_abort(now) {
            MaybeTimeout::Timeout(finished_game) => GamePlayerActionResult::Timeout(finished_game),
            MaybeTimeout::Result(Some(finished_game)) => {
                GamePlayerActionResult::Result(Some(finished_game))
            }
            MaybeTimeout::Result(None) => GamePlayerActionResult::Result(None),
        }
    }
    pub fn abort_forced(&mut self, now: SystemTime) -> GamePlayerActionResult<TakFinishedGame> {
        match self.game.abort_forced(now) {
            MaybeTimeout::Timeout(finished_game) => GamePlayerActionResult::Timeout(finished_game),
            MaybeTimeout::Result(finished_game) => GamePlayerActionResult::Result(finished_game),
        }
    }

    pub fn set_request(
        &mut self,
        player: TakPlayer,
        request: GameRequest,
        now: SystemTime,
    ) -> GamePlayerActionResult<Option<(GameRequest, TakTimeInfo)>> {
        match self.game.check_timeout(now) {
            MaybeTimeout::Timeout(finished_game) => GamePlayerActionResult::Timeout(finished_game),
            MaybeTimeout::Result(()) => {
                let requests = match player {
                    TakPlayer::White => &mut self.white_requests,
                    TakPlayer::Black => &mut self.black_requests,
                };
                match requests.set_request(request) {
                    Some(request) => {
                        let time_info = self.game.get_time_info(now);
                        GamePlayerActionResult::Result(Some((request, time_info)))
                    }
                    None => GamePlayerActionResult::Result(None),
                }
            }
        }
    }
    pub fn accept_draw_request(
        &mut self,
        player: TakPlayer,
        now: SystemTime,
    ) -> GamePlayerActionResult<Option<(GameRequest, TakFinishedGame)>> {
        match self.game.check_timeout(now) {
            MaybeTimeout::Timeout(finished_game) => {
                return GamePlayerActionResult::Timeout(finished_game);
            }
            MaybeTimeout::Result(()) => {}
        };
        let requests = match player.opponent() {
            TakPlayer::White => &mut self.white_requests,
            TakPlayer::Black => &mut self.black_requests,
        };
        match requests.consume_request(GameRequestType::Draw) {
            GameRequest::Draw(true) => match self.game.agree_draw(now) {
                MaybeTimeout::Timeout(finished_game) => {
                    GamePlayerActionResult::Timeout(finished_game)
                }
                MaybeTimeout::Result(finished_game) => {
                    GamePlayerActionResult::Result(Some((GameRequest::Draw(false), finished_game)))
                }
            },
            _ => GamePlayerActionResult::Result(None),
        }
    }
    pub fn accept_undo_request(
        &mut self,
        player: TakPlayer,
        now: SystemTime,
    ) -> GamePlayerActionResult<Option<(GameRequest, TakTimeInfo, Option<GameUndoActionRecord>)>>
    {
        let did_undo = match self.game.check_timeout(now) {
            MaybeTimeout::Timeout(finished_game) => {
                return GamePlayerActionResult::Timeout(finished_game);
            }
            MaybeTimeout::Result(()) => {
                let requests = match player.opponent() {
                    TakPlayer::White => &mut self.white_requests,
                    TakPlayer::Black => &mut self.black_requests,
                };
                match requests.consume_request(GameRequestType::Undo) {
                    GameRequest::Undo(true) => match self.game.undo_action(now) {
                        Ok(MaybeTimeout::Timeout(finished_game)) => {
                            return GamePlayerActionResult::Timeout(finished_game);
                        }
                        Ok(MaybeTimeout::Result(did_undo)) => did_undo,
                        Err(_) => {
                            return GamePlayerActionResult::Result(None);
                        }
                    },
                    _ => return GamePlayerActionResult::Result(None),
                }
            }
        };

        let time_info = self.game.get_time_info(now);

        let undo_record = if did_undo {
            let ply_index = self.game.ply_index();
            Some(GameUndoActionRecord::new(ply_index))
        } else {
            None
        };
        GamePlayerActionResult::Result(Some((GameRequest::Undo(false), time_info, undo_record)))
    }
    pub fn accept_more_time_request(
        &mut self,
        player: TakPlayer,
        now: SystemTime,
    ) -> GamePlayerActionResult<Option<(GameRequest, TakTimeInfo)>> {
        match self.game.check_timeout(now) {
            MaybeTimeout::Timeout(finished_game) => {
                return GamePlayerActionResult::Timeout(finished_game);
            }
            MaybeTimeout::Result(()) => {
                let requests = match player.opponent() {
                    TakPlayer::White => &mut self.white_requests,
                    TakPlayer::Black => &mut self.black_requests,
                };
                match requests.consume_request(GameRequestType::MoreTime) {
                    GameRequest::MoreTime(Some(duration)) => {
                        match self.game.give_time_to_player(player, duration, now) {
                            MaybeTimeout::Timeout(finished_game) => {
                                return GamePlayerActionResult::Timeout(finished_game);
                            }
                            MaybeTimeout::Result(()) => {}
                        }
                    }
                    _ => return GamePlayerActionResult::Result(None),
                }
            }
        };
        let time_info = self.game.get_time_info(now);
        GamePlayerActionResult::Result(Some((GameRequest::MoreTime(None), time_info)))
    }
    pub fn check_timeout(&mut self, now: SystemTime) -> CheckTimeoutResult {
        match self.game.check_timeout(now) {
            MaybeTimeout::Timeout(finished_game) => CheckTimeoutResult::TimedOut(finished_game),
            MaybeTimeout::Result(()) => {
                let time_info = self.game.get_time_info(now);
                CheckTimeoutResult::NoTimeout(time_info)
            }
        }
    }
    pub fn check_disconnect_timeout(
        &mut self,
        player: TakPlayer,
        disconnected_duration: Duration,
        now: SystemTime,
    ) -> GamePlayerActionResult<CheckDisconnectTimeoutResult> {
        if matches!(self.game.time_settings, TakTimeSettings::Async(_)) {
            return GamePlayerActionResult::Result(CheckDisconnectTimeoutResult::CantTimeOut);
        }
        let timeout_duration = Duration::from_secs(60 * 5);
        if disconnected_duration < timeout_duration {
            return GamePlayerActionResult::Result(CheckDisconnectTimeoutResult::NoTimeout(
                timeout_duration - disconnected_duration,
            ));
        }
        match self.game.resign_or_abandon(player, now) {
            MaybeTimeout::Timeout(finished_game) => GamePlayerActionResult::Timeout(finished_game),
            MaybeTimeout::Result(finished_game) => GamePlayerActionResult::Result(
                CheckDisconnectTimeoutResult::TimedOut(finished_game),
            ),
        }
    }
}
