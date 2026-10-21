use std::time::{Duration, Instant};

use crate::{
    MatchId, UserId,
    game::request::{GameRequest, GameRequestSystem, GameRequestType},
};
use nonempty::NonEmpty;
use tak_core::{
    MaybeTimeout, TakAction, TakFinishedGame, TakGameResult, TakGameSettings, TakOngoingGame,
    TakPlayer, TakTimeInfo, TakTimeSettings,
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

#[derive(Debug, Clone)]
pub struct GameRatingInfo {
    pub rating_change_white: f64,
    pub rating_change_black: f64,
}

#[derive(Clone, Debug)]
pub struct GameEvent {
    pub event_type: GameEventType,
    pub date: chrono::DateTime<chrono::Utc>,
    pub time_info: TakTimeInfo,
}

impl GameEvent {
    pub fn new(event_type: GameEventType, time_info: TakTimeInfo) -> Self {
        Self {
            event_type,
            date: chrono::Utc::now(),
            time_info,
        }
    }
}

#[derive(Clone, Debug)]
pub enum GameEventType {
    Start,
    Action {
        action: TakAction,
    },
    RequestSet {
        player: TakPlayer,
        request: GameRequest,
    },
    ActionUndone,
    TimeGiven {
        player: TakPlayer,
        duration: Duration,
    },
    GameOver(GameOverEventType),
}

#[derive(Clone, Debug)]
pub enum GameOverEventType {
    Action,
    Timeout,
    Resignation,
    Abandonment,
    DrawAgreement,
    Aborted,
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
pub struct PlayerGameRequest {
    pub user_id: UserId,
    pub request: GameRequest,
}

#[derive(Clone, Debug)]
pub enum Game {
    Ongoing(OngoingGame),
    Finished(FinishedGame),
}

#[derive(Clone, Debug)]
pub struct OngoingGame {
    pub metadata: GameMetadata,
    pub game: TakOngoingGame,
    pub requests: GameRequestSystem,
    pub events: NonEmpty<GameEvent>,
}

#[derive(Clone, Debug)]
pub struct FinishedGame {
    pub metadata: GameMetadata,
    pub result: TakGameResult,
    pub rating_info: Option<GameRatingInfo>,
    pub events: NonEmpty<GameEvent>,
}

impl FinishedGame {
    pub fn new(
        metadata: GameMetadata,
        game: TakFinishedGame,
        rating_info: Option<GameRatingInfo>,
        events: NonEmpty<GameEvent>,
    ) -> Self {
        Self {
            metadata,
            result: game.game_result().clone(),
            rating_info,
            events,
        }
    }
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
    NotAPlayerInGame,
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

impl OngoingGame {
    pub fn new(metadata: GameMetadata) -> Self {
        let now = Instant::now();
        let game = TakOngoingGame::new(metadata.settings.clone(), now);
        let time_info = game.get_time_info(now);
        Self {
            metadata,
            game,
            requests: GameRequestSystem::new(),
            events: NonEmpty::new(GameEvent::new(GameEventType::Start, time_info)),
        }
    }
    pub fn get_time_info(&self, now: Instant) -> TakTimeInfo {
        self.game.get_time_info(now)
    }

    pub fn do_action(
        &mut self,
        user_id: UserId,
        action: TakAction,
        now: Instant,
    ) -> GamePlayerActionResult<DoActionResult> {
        let Some(player) = self.metadata.get_player(user_id) else {
            return GamePlayerActionResult::NotAPlayerInGame;
        };
        if self.game.current_player() != player {
            return GamePlayerActionResult::Result(DoActionResult::NotPlayersTurn);
        }
        match self.game.do_action(action.clone(), now) {
            Err(e) => GamePlayerActionResult::Result(DoActionResult::InvalidAction(e)),
            Ok(MaybeTimeout::Timeout(finished_game)) => {
                let time_info = finished_game.get_time_info();
                self.events.push(GameEvent::new(
                    GameEventType::GameOver(GameOverEventType::Timeout),
                    time_info,
                ));
                GamePlayerActionResult::Timeout(finished_game)
            }
            Ok(MaybeTimeout::Result(Some(finished_game))) => {
                let ply_index = finished_game.action_history().len();
                let time_info = finished_game.get_time_info();
                self.events.push(GameEvent::new(
                    GameEventType::Action {
                        action: action.clone(),
                    },
                    time_info.clone(),
                ));
                self.events.push(GameEvent::new(
                    GameEventType::GameOver(GameOverEventType::Action),
                    time_info.clone(),
                ));
                GamePlayerActionResult::Result(DoActionResult::GameOver(
                    finished_game,
                    GameActionRecord::new(action, ply_index, time_info),
                ))
            }
            Ok(MaybeTimeout::Result(None)) => {
                let ply_index = self.game.action_history().len();
                let time_info = self.game.get_time_info(now);
                self.events.push(GameEvent::new(
                    GameEventType::Action {
                        action: action.clone(),
                    },
                    time_info.clone(),
                ));
                GamePlayerActionResult::Result(DoActionResult::ActionPerformed(
                    GameActionRecord::new(action, ply_index, time_info),
                ))
            }
        }
    }
    pub fn resign(
        &mut self,
        user_id: UserId,
        now: Instant,
    ) -> GamePlayerActionResult<TakFinishedGame> {
        let Some(player) = self.metadata.get_player(user_id) else {
            return GamePlayerActionResult::NotAPlayerInGame;
        };
        match self.game.resign_or_abandon(player, now) {
            MaybeTimeout::Timeout(finished_game) => {
                let time_info = finished_game.get_time_info();
                self.events.push(GameEvent::new(
                    GameEventType::GameOver(GameOverEventType::Timeout),
                    time_info,
                ));
                GamePlayerActionResult::Timeout(finished_game)
            }
            MaybeTimeout::Result(finished_game) => {
                let time_info = finished_game.get_time_info();
                self.events.push(GameEvent::new(
                    GameEventType::GameOver(GameOverEventType::Resignation),
                    time_info,
                ));
                GamePlayerActionResult::Result(finished_game)
            }
        }
    }
    pub fn try_abort(
        &mut self,
        user_id: UserId,
        now: Instant,
    ) -> GamePlayerActionResult<Option<TakFinishedGame>> {
        let Some(_) = self.metadata.get_player(user_id) else {
            return GamePlayerActionResult::NotAPlayerInGame;
        };
        match self.game.try_abort(now) {
            MaybeTimeout::Timeout(finished_game) => {
                let time_info = finished_game.get_time_info();
                self.events.push(GameEvent::new(
                    GameEventType::GameOver(GameOverEventType::Timeout),
                    time_info,
                ));
                GamePlayerActionResult::Timeout(finished_game)
            }
            MaybeTimeout::Result(Some(finished_game)) => {
                let time_info = finished_game.get_time_info();
                self.events.push(GameEvent::new(
                    GameEventType::GameOver(GameOverEventType::Aborted),
                    time_info,
                ));
                GamePlayerActionResult::Result(Some(finished_game))
            }
            MaybeTimeout::Result(None) => GamePlayerActionResult::Result(None),
        }
    }
    pub fn abort_forced(&mut self, now: Instant) -> GamePlayerActionResult<TakFinishedGame> {
        match self.game.abort_forced(now) {
            MaybeTimeout::Timeout(finished_game) => {
                let time_info = finished_game.get_time_info();
                self.events.push(GameEvent::new(
                    GameEventType::GameOver(GameOverEventType::Timeout),
                    time_info,
                ));
                GamePlayerActionResult::Timeout(finished_game)
            }
            MaybeTimeout::Result(finished_game) => {
                let time_info = finished_game.get_time_info();
                self.events.push(GameEvent::new(
                    GameEventType::GameOver(GameOverEventType::Aborted),
                    time_info,
                ));
                GamePlayerActionResult::Result(finished_game)
            }
        }
    }

    pub fn set_request(
        &mut self,
        user_id: UserId,
        request: GameRequest,
        now: Instant,
    ) -> GamePlayerActionResult<Option<(PlayerGameRequest, TakTimeInfo)>> {
        let Some(player) = self.metadata.get_player(user_id) else {
            return GamePlayerActionResult::NotAPlayerInGame;
        };
        match self.game.check_timeout(now) {
            MaybeTimeout::Timeout(finished_game) => {
                let time_info = finished_game.get_time_info();
                self.events.push(GameEvent::new(
                    GameEventType::GameOver(GameOverEventType::Timeout),
                    time_info,
                ));
                GamePlayerActionResult::Timeout(finished_game)
            }
            MaybeTimeout::Result(()) => match self.requests.set_request(player, request.clone()) {
                Some(request) => {
                    let time_info = self.game.get_time_info(now);
                    self.events.push(GameEvent::new(
                        GameEventType::RequestSet {
                            request: request.clone(),
                            player,
                        },
                        time_info.clone(),
                    ));
                    GamePlayerActionResult::Result(Some((
                        PlayerGameRequest { user_id, request },
                        time_info,
                    )))
                }
                None => GamePlayerActionResult::Result(None),
            },
        }
    }
    pub fn accept_draw_request(
        &mut self,
        user_id: UserId,
        now: Instant,
    ) -> GamePlayerActionResult<Option<(PlayerGameRequest, TakTimeInfo, TakFinishedGame)>> {
        let Some(player) = self.metadata.get_player(user_id) else {
            return GamePlayerActionResult::NotAPlayerInGame;
        };
        match self.game.check_timeout(now) {
            MaybeTimeout::Timeout(finished_game) => {
                let time_info = finished_game.get_time_info();
                self.events.push(GameEvent::new(
                    GameEventType::GameOver(GameOverEventType::Timeout),
                    time_info,
                ));
                return GamePlayerActionResult::Timeout(finished_game);
            }
            MaybeTimeout::Result(()) => {}
        };
        match self
            .requests
            .consume_request(player.opponent(), GameRequestType::Draw)
        {
            GameRequest::Draw(true) => match self.game.agree_draw(now) {
                MaybeTimeout::Timeout(finished_game) => {
                    let time_info = finished_game.get_time_info();
                    self.events.push(GameEvent::new(
                        GameEventType::GameOver(GameOverEventType::Timeout),
                        time_info,
                    ));
                    GamePlayerActionResult::Timeout(finished_game)
                }
                MaybeTimeout::Result(finished_game) => {
                    let time_info = finished_game.get_time_info();
                    self.events.push(GameEvent::new(
                        GameEventType::GameOver(GameOverEventType::DrawAgreement),
                        time_info.clone(),
                    ));
                    let opponent_id = self.metadata.get_user_id(player.opponent());
                    GamePlayerActionResult::Result(Some((
                        PlayerGameRequest {
                            user_id: opponent_id,
                            request: GameRequest::Draw(false),
                        },
                        time_info,
                        finished_game,
                    )))
                }
            },
            _ => GamePlayerActionResult::Result(None),
        }
    }
    pub fn accept_undo_request(
        &mut self,
        user_id: UserId,
        now: Instant,
    ) -> GamePlayerActionResult<
        Option<(PlayerGameRequest, TakTimeInfo, Option<GameUndoActionRecord>)>,
    > {
        let Some(player) = self.metadata.get_player(user_id) else {
            return GamePlayerActionResult::NotAPlayerInGame;
        };
        let did_undo = match self.game.check_timeout(now) {
            MaybeTimeout::Timeout(finished_game) => {
                let time_info = finished_game.get_time_info();
                self.events.push(GameEvent::new(
                    GameEventType::GameOver(GameOverEventType::Timeout),
                    time_info,
                ));
                return GamePlayerActionResult::Timeout(finished_game);
            }
            MaybeTimeout::Result(()) => match self
                .requests
                .consume_request(player.opponent(), GameRequestType::Undo)
            {
                GameRequest::Undo(true) => match self.game.undo_action(now) {
                    MaybeTimeout::Timeout(finished_game) => {
                        return GamePlayerActionResult::Timeout(finished_game);
                    }
                    MaybeTimeout::Result(did_undo) => did_undo,
                },
                _ => return GamePlayerActionResult::Result(None),
            },
        };

        let time_info = self.game.get_time_info(now);

        let undo_record = if did_undo {
            let ply_index = self.game.action_history().len();
            self.events.push(GameEvent::new(
                GameEventType::ActionUndone,
                time_info.clone(),
            ));
            Some(GameUndoActionRecord::new(ply_index))
        } else {
            None
        };
        let opponent_id = self.metadata.get_user_id(player.opponent());
        GamePlayerActionResult::Result(Some((
            PlayerGameRequest {
                user_id: opponent_id,
                request: GameRequest::Undo(false),
            },
            time_info,
            undo_record,
        )))
    }
    pub fn accept_more_time_request(
        &mut self,
        user_id: UserId,
        now: Instant,
    ) -> GamePlayerActionResult<Option<(PlayerGameRequest, TakTimeInfo)>> {
        let Some(player) = self.metadata.get_player(user_id) else {
            return GamePlayerActionResult::NotAPlayerInGame;
        };
        match self.game.check_timeout(now) {
            MaybeTimeout::Timeout(finished_game) => {
                let time_info = finished_game.get_time_info();
                self.events.push(GameEvent::new(
                    GameEventType::GameOver(GameOverEventType::Timeout),
                    time_info,
                ));
                return GamePlayerActionResult::Timeout(finished_game);
            }
            MaybeTimeout::Result(()) => match self
                .requests
                .consume_request(player.opponent(), GameRequestType::MoreTime)
            {
                GameRequest::MoreTime(Some(duration)) => {
                    match self.game.give_time_to_player(player, duration, now) {
                        MaybeTimeout::Timeout(finished_game) => {
                            return GamePlayerActionResult::Timeout(finished_game);
                        }
                        MaybeTimeout::Result(()) => {}
                    }
                }
                _ => return GamePlayerActionResult::Result(None),
            },
        };
        let time_info = self.game.get_time_info(now);
        let opponent_id = self.metadata.get_user_id(player.opponent());
        GamePlayerActionResult::Result(Some((
            PlayerGameRequest {
                user_id: opponent_id,
                request: GameRequest::MoreTime(None),
            },
            time_info,
        )))
    }
    pub fn check_timeout(&mut self, now: Instant) -> CheckTimeoutResult {
        match self.game.check_timeout(now) {
            MaybeTimeout::Timeout(finished_game) => {
                let time_info = finished_game.get_time_info();
                self.events.push(GameEvent::new(
                    GameEventType::GameOver(GameOverEventType::Timeout),
                    time_info,
                ));
                CheckTimeoutResult::TimedOut(finished_game)
            }
            MaybeTimeout::Result(()) => {
                let time_info = self.game.get_time_info(now);
                CheckTimeoutResult::NoTimeout(time_info)
            }
        }
    }
    pub fn check_disconnect_timeout(
        &mut self,
        user_id: UserId,
        disconnected_duration: Duration,
        now: Instant,
    ) -> GamePlayerActionResult<CheckDisconnectTimeoutResult> {
        let Some(player) = self.metadata.get_player(user_id) else {
            return GamePlayerActionResult::NotAPlayerInGame;
        };

        if matches!(
            self.metadata.settings.time_settings,
            TakTimeSettings::Async(_)
        ) {
            return GamePlayerActionResult::Result(CheckDisconnectTimeoutResult::CantTimeOut);
        }
        let timeout_duration = Duration::from_secs(60 * 5);
        if disconnected_duration < timeout_duration {
            return GamePlayerActionResult::Result(CheckDisconnectTimeoutResult::NoTimeout(
                timeout_duration - disconnected_duration,
            ));
        }
        let finished_game = match self.game.resign_or_abandon(player, now) {
            MaybeTimeout::Timeout(finished_game) => {
                let time_info = finished_game.get_time_info();
                self.events.push(GameEvent::new(
                    GameEventType::GameOver(GameOverEventType::Timeout),
                    time_info,
                ));
                return GamePlayerActionResult::Timeout(finished_game);
            }
            MaybeTimeout::Result(finished_game) => finished_game,
        };

        let time_info = finished_game.get_time_info();
        self.events.push(GameEvent::new(
            GameEventType::GameOver(GameOverEventType::Abandonment),
            time_info,
        ));
        GamePlayerActionResult::Result(CheckDisconnectTimeoutResult::TimedOut(finished_game))
    }
}
