use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use crate::domain::{
    GameId, MatchId, PlayerId,
    game::request::{GameRequest, GameRequestSystem, GameRequestType},
};
use dashmap::DashMap;
use tak_core::{
    MaybeTimeout, TakAction, TakFinishedGame, TakGameSettings, TakOngoingGame, TakPlayer,
    TakTimeInfo, TakTimeSettings,
};

pub mod request;

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
    pub white_id: PlayerId,
    pub black_id: PlayerId,
    pub settings: TakGameSettings,
    pub is_rated: bool,
    pub match_id: Option<MatchId>,
}

impl GameMetadata {
    pub fn get_opponent(&self, player: PlayerId) -> Option<PlayerId> {
        if player == self.white_id {
            Some(self.black_id)
        } else if player == self.black_id {
            Some(self.white_id)
        } else {
            None
        }
    }
    pub fn get_player_id(&self, player: TakPlayer) -> PlayerId {
        match player {
            TakPlayer::White => self.white_id,
            TakPlayer::Black => self.black_id,
        }
    }

    pub fn get_player(&self, id: PlayerId) -> Option<TakPlayer> {
        if id == self.white_id {
            Some(TakPlayer::White)
        } else if id == self.black_id {
            Some(TakPlayer::Black)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
pub struct PlayerGameRequest {
    pub player_id: PlayerId,
    pub request: GameRequest,
}

#[derive(Clone, Debug)]
pub struct OngoingGame {
    pub game_id: GameId,
    pub metadata: GameMetadata,
    pub game: TakOngoingGame,
    pub requests: GameRequestSystem,
    pub events: Vec<GameEvent>,
}

impl OngoingGame {
    pub fn get_time_info(&self, now: Instant) -> TakTimeInfo {
        self.game.get_time_info(now)
    }
}

#[derive(Clone, Debug)]
pub struct FinishedGame {
    pub game_id: GameId,
    pub metadata: GameMetadata,
    pub game: TakFinishedGame,
    pub events: Vec<GameEvent>,
}

impl FinishedGame {
    fn new(game: &OngoingGame, tak_game: TakFinishedGame) -> Self {
        Self {
            game_id: game.game_id,
            metadata: game.metadata.clone(),
            game: tak_game,
            events: game.events.clone(),
        }
    }
}

pub trait GameService {
    fn create_game_metadata(
        &self,
        date: chrono::DateTime<chrono::Utc>,
        white_id: PlayerId,
        black_id: PlayerId,
        is_rated: bool,
        game_settings: TakGameSettings,
        match_id: Option<MatchId>,
    ) -> GameMetadata;
    fn create_game(&self, id: GameId, metadata: GameMetadata) -> OngoingGame;
    fn get_game_by_id(&self, game_id: GameId) -> Option<OngoingGame>;
    fn get_games(&self) -> impl Iterator<Item = OngoingGame>;
    fn abort_all_games(&self, now: Instant) -> Vec<FinishedGame>;
    fn check_timeout(&self, game_id: GameId, now: Instant) -> CheckTimeoutResult;
    fn check_disconnect_timeout(
        &self,
        game_id: GameId,
        player: PlayerId,
        disconnected_duration: Duration,
        now: Instant,
    ) -> GamePlayerActionResult<CheckDisconnectTimeoutResult>;
    fn do_action(
        &self,
        game_id: GameId,
        player: PlayerId,
        action: TakAction,
        now: Instant,
    ) -> GamePlayerActionResult<DoActionResult>;
    fn resign(
        &self,
        game_id: GameId,
        player: PlayerId,
        now: Instant,
    ) -> GamePlayerActionResult<FinishedGame>;
    fn abort(
        &self,
        game_id: GameId,
        player: PlayerId,
        now: Instant,
    ) -> GamePlayerActionResult<Option<FinishedGame>>;
    fn set_request(
        &self,
        game_id: GameId,
        player: PlayerId,
        request: GameRequest,
        now: Instant,
    ) -> GamePlayerActionResult<Option<(PlayerGameRequest, TakTimeInfo)>>;
    fn accept_draw_request(
        &self,
        game_id: GameId,
        player: PlayerId,
        now: Instant,
    ) -> GamePlayerActionResult<Option<(PlayerGameRequest, TakTimeInfo, FinishedGame)>>;
    fn accept_undo_request(
        &self,
        game_id: GameId,
        player: PlayerId,
        now: Instant,
    ) -> GamePlayerActionResult<
        Option<(PlayerGameRequest, TakTimeInfo, Option<GameUndoActionRecord>)>,
    >;
    fn accept_more_time_request(
        &self,
        game_id: GameId,
        player: PlayerId,
        now: Instant,
    ) -> GamePlayerActionResult<Option<(PlayerGameRequest, TakTimeInfo)>>;
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
    GameOver(GameActionRecord, FinishedGame),
    NotPlayersTurn,
    InvalidAction(tak_core::InvalidActionReason),
}

pub enum GamePlayerActionResult<R> {
    GameNotFound,
    NotAPlayerInGame,
    Timeout(FinishedGame),
    Result(R),
}

pub enum CheckTimeoutResult {
    GameNotFound,
    TimedOut(FinishedGame),
    NoTimeout(TakTimeInfo),
}

pub enum CheckDisconnectTimeoutResult {
    TimedOut(FinishedGame),
    CantTimeOut,
    NoTimeout(Duration),
}

pub struct GameServiceImpl {
    games: Arc<DashMap<GameId, OngoingGame>>,
}

enum GameControl {
    Keep,
    Remove,
}

impl GameServiceImpl {
    pub fn new() -> Self {
        Self {
            games: Arc::new(DashMap::new()),
        }
    }

    fn with_game_might_end<F, R>(&self, game_id: GameId, f: F) -> Option<R>
    where
        F: FnOnce(&mut OngoingGame) -> (GameControl, R),
    {
        let mut res = None;
        self.games.remove_if_mut(&game_id, |_, entry| {
            let (new_game, r) = f(entry);
            res = Some(r);
            match new_game {
                GameControl::Keep => false,
                GameControl::Remove => true,
            }
        });
        res
    }

    fn game_player_action<R, FR>(
        &self,
        game_id: GameId,
        player: PlayerId,
        action_fn: impl FnOnce(
            &mut OngoingGame,
            TakPlayer,
        ) -> Result<MaybeTimeout<FR, TakFinishedGame>, R>,
        decision_fn: impl FnOnce(&mut OngoingGame, TakPlayer, FR) -> (GameControl, R),
    ) -> GamePlayerActionResult<R> {
        self.with_game_might_end(game_id, |game_entry| {
            let current_player = match game_entry.metadata.get_player(player) {
                Some(p) => p,
                None => return (GameControl::Keep, GamePlayerActionResult::NotAPlayerInGame),
            };
            let action_res = action_fn(game_entry, current_player);
            match action_res {
                Ok(MaybeTimeout::Timeout(finished_game)) => {
                    let time_info = finished_game.get_time_info();
                    game_entry.events.push(GameEvent::new(
                        GameEventType::GameOver(GameOverEventType::Timeout),
                        time_info,
                    ));
                    let finished_game = FinishedGame::new(game_entry, finished_game);
                    (
                        GameControl::Remove,
                        GamePlayerActionResult::Timeout(finished_game),
                    )
                }
                Ok(MaybeTimeout::Result(result)) => {
                    let (control, re) = decision_fn(game_entry, current_player, result);
                    (control, GamePlayerActionResult::Result(re))
                }
                Err(e) => (GameControl::Keep, GamePlayerActionResult::Result(e)),
            }
        })
        .unwrap_or(GamePlayerActionResult::GameNotFound)
    }
}

impl GameService for GameServiceImpl {
    fn create_game_metadata(
        &self,
        date: chrono::DateTime<chrono::Utc>,
        white_id: PlayerId,
        black_id: PlayerId,
        is_rated: bool,
        game_settings: TakGameSettings,
        match_id: Option<MatchId>,
    ) -> GameMetadata {
        GameMetadata {
            date,
            white_id,
            black_id,
            settings: game_settings,
            is_rated,
            match_id,
        }
    }
    fn create_game(&self, id: GameId, metadata: GameMetadata) -> OngoingGame {
        let game = TakOngoingGame::new(metadata.settings.clone());

        let game_struct = OngoingGame {
            game_id: id,
            game,
            metadata,
            requests: GameRequestSystem::new(),
            events: Vec::new(),
        };
        self.games.insert(id, game_struct.clone());

        game_struct
    }

    fn get_game_by_id(&self, game_id: GameId) -> Option<OngoingGame> {
        self.games.get(&game_id).map(|entry| entry.clone())
    }

    fn get_games(&self) -> impl Iterator<Item = OngoingGame> {
        self.games.iter().map(|entry| entry.clone())
    }

    fn do_action(
        &self,
        game_id: GameId,
        player: PlayerId,
        action: TakAction,
        now: Instant,
    ) -> GamePlayerActionResult<DoActionResult> {
        self.game_player_action(
            game_id,
            player,
            |game_entry, player| {
                if game_entry.game.current_player() != player {
                    Err(DoActionResult::NotPlayersTurn)
                } else {
                    game_entry
                        .game
                        .do_action(action.clone(), now)
                        .map_err(|e| DoActionResult::InvalidAction(e))
                }
            },
            |game_entry, _, res| match res {
                Some(finished_game) => {
                    let ply_index = finished_game.action_history().len();
                    let time_info = finished_game.get_time_info();
                    game_entry.events.push(GameEvent::new(
                        GameEventType::Action {
                            action: action.clone(),
                        },
                        time_info.clone(),
                    ));
                    game_entry.events.push(GameEvent::new(
                        GameEventType::GameOver(GameOverEventType::Action),
                        time_info.clone(),
                    ));
                    let finished_game = FinishedGame::new(game_entry, finished_game);
                    (
                        GameControl::Remove,
                        DoActionResult::GameOver(
                            GameActionRecord::new(action.clone(), ply_index, time_info),
                            finished_game,
                        ),
                    )
                }
                None => {
                    let ply_index = game_entry.game.action_history().len();
                    let time_info = game_entry.game.get_time_info(now);
                    game_entry.events.push(GameEvent::new(
                        GameEventType::Action {
                            action: action.clone(),
                        },
                        time_info.clone(),
                    ));
                    (
                        GameControl::Keep,
                        DoActionResult::ActionPerformed(GameActionRecord::new(
                            action.clone(),
                            ply_index,
                            time_info,
                        )),
                    )
                }
            },
        )
    }

    fn resign(
        &self,
        game_id: GameId,
        player: PlayerId,
        now: Instant,
    ) -> GamePlayerActionResult<FinishedGame> {
        self.game_player_action(
            game_id,
            player,
            |game_entry, current_player| Ok(game_entry.game.resign_or_abandon(current_player, now)),
            |game_entry, _, finished_game| {
                let time_info = finished_game.get_time_info();
                game_entry.events.push(GameEvent::new(
                    GameEventType::GameOver(GameOverEventType::Resignation),
                    time_info,
                ));
                let finished_game = FinishedGame::new(game_entry, finished_game);
                (GameControl::Remove, finished_game)
            },
        )
    }

    fn abort(
        &self,
        game_id: GameId,
        player: PlayerId,
        now: Instant,
    ) -> GamePlayerActionResult<Option<FinishedGame>> {
        self.game_player_action(
            game_id,
            player,
            |game_entry, current_player| Ok(game_entry.game.abort(now, current_player)),
            |game_entry, _, res| match res {
                Some(finished_game) => {
                    let time_info = finished_game.get_time_info();
                    game_entry.events.push(GameEvent::new(
                        GameEventType::GameOver(GameOverEventType::Aborted),
                        time_info,
                    ));
                    let finished_game = FinishedGame::new(game_entry, finished_game);
                    (GameControl::Remove, Some(finished_game))
                }
                None => (GameControl::Keep, None),
            },
        )
    }

    fn abort_all_games(&self, now: Instant) -> Vec<FinishedGame> {
        let mut finished_games = Vec::new();
        self.games.retain(|_, game_entry| {
            // Doesn't matter whether the game ended due to timeout or due to abort.
            let finished_game = match game_entry.game.abort_forced(now) {
                MaybeTimeout::Result(game) => game,
                MaybeTimeout::Timeout(game) => game,
            };
            let time_info = finished_game.get_time_info();
            game_entry.events.push(GameEvent::new(
                GameEventType::GameOver(GameOverEventType::Aborted),
                time_info,
            ));
            let finished_game = FinishedGame::new(game_entry, finished_game);
            finished_games.push(finished_game);
            false
        });
        finished_games
    }

    fn set_request(
        &self,
        game_id: GameId,
        player: PlayerId,
        request: GameRequest,
        now: Instant,
    ) -> GamePlayerActionResult<Option<(PlayerGameRequest, TakTimeInfo)>> {
        self.game_player_action(
            game_id,
            player,
            |game_entry, current_player| match game_entry.game.check_timeout(now) {
                MaybeTimeout::Timeout(game) => Ok(MaybeTimeout::Timeout(game)),
                MaybeTimeout::Result(()) => Ok(MaybeTimeout::Result(
                    game_entry
                        .requests
                        .set_request(current_player, request.clone()),
                )),
            },
            |game_entry, current_player, res| match res {
                Some(request) => {
                    let time_info = game_entry.game.get_time_info(now);
                    game_entry.events.push(GameEvent::new(
                        GameEventType::RequestSet {
                            request: request.clone(),
                            player: current_player,
                        },
                        time_info.clone(),
                    ));
                    (
                        GameControl::Keep,
                        Some((
                            PlayerGameRequest {
                                player_id: player,
                                request,
                            },
                            time_info,
                        )),
                    )
                }
                None => (GameControl::Keep, None),
            },
        )
    }

    fn accept_draw_request(
        &self,
        game_id: GameId,
        player: PlayerId,
        now: Instant,
    ) -> GamePlayerActionResult<Option<(PlayerGameRequest, TakTimeInfo, FinishedGame)>> {
        self.game_player_action(
            game_id,
            player,
            |game_entry, current_player| match game_entry.game.check_timeout(now) {
                MaybeTimeout::Timeout(game) => Ok(MaybeTimeout::Timeout(game)),
                MaybeTimeout::Result(()) => Ok(MaybeTimeout::Result(
                    match game_entry
                        .requests
                        .consume_request(current_player.opponent(), GameRequestType::Draw)
                    {
                        GameRequest::Draw(true) => match game_entry.game.agree_draw(now) {
                            MaybeTimeout::Timeout(finished_game) => {
                                return Ok(MaybeTimeout::Timeout(finished_game));
                            }
                            MaybeTimeout::Result(finished_game) => Some(finished_game),
                        },
                        _ => None,
                    },
                )),
            },
            |game_entry, current_player, res| match res {
                Some(finished_game) => {
                    let time_info = finished_game.get_time_info();
                    game_entry.events.push(GameEvent::new(
                        GameEventType::GameOver(GameOverEventType::DrawAgreement),
                        time_info.clone(),
                    ));
                    let finished_game = FinishedGame::new(game_entry, finished_game);
                    let opponent_id = game_entry.metadata.get_player_id(current_player.opponent());
                    (
                        GameControl::Remove,
                        Some((
                            PlayerGameRequest {
                                player_id: opponent_id,
                                request: GameRequest::Draw(false),
                            },
                            time_info,
                            finished_game,
                        )),
                    )
                }
                None => (GameControl::Keep, None),
            },
        )
    }
    fn accept_undo_request(
        &self,
        game_id: GameId,
        player: PlayerId,
        now: Instant,
    ) -> GamePlayerActionResult<
        Option<(PlayerGameRequest, TakTimeInfo, Option<GameUndoActionRecord>)>,
    > {
        self.game_player_action(
            game_id,
            player,
            |game_entry, current_player| match game_entry.game.check_timeout(now) {
                MaybeTimeout::Timeout(game) => Ok(MaybeTimeout::Timeout(game)),
                MaybeTimeout::Result(()) => Ok(MaybeTimeout::Result(
                    match game_entry
                        .requests
                        .consume_request(current_player.opponent(), GameRequestType::Undo)
                    {
                        GameRequest::Undo(true) => match game_entry.game.undo_action(now) {
                            MaybeTimeout::Timeout(finished_game) => {
                                return Ok(MaybeTimeout::Timeout(finished_game));
                            }
                            MaybeTimeout::Result(did_undo) => Some(did_undo),
                        },
                        _ => None,
                    },
                )),
            },
            |game_entry, current_player, res| match res {
                Some(did_undo) => {
                    let time_info = game_entry.game.get_time_info(now);

                    let undo_record = if did_undo {
                        let ply_index = game_entry.game.action_history().len();
                        game_entry.events.push(GameEvent::new(
                            GameEventType::ActionUndone,
                            time_info.clone(),
                        ));
                        Some(GameUndoActionRecord::new(ply_index))
                    } else {
                        None
                    };
                    let opponent_id = game_entry.metadata.get_player_id(current_player.opponent());
                    (
                        GameControl::Keep,
                        Some((
                            PlayerGameRequest {
                                player_id: opponent_id,
                                request: GameRequest::Undo(false),
                            },
                            time_info,
                            undo_record,
                        )),
                    )
                }
                None => (GameControl::Keep, None),
            },
        )
    }

    fn accept_more_time_request(
        &self,
        game_id: GameId,
        player: PlayerId,
        now: Instant,
    ) -> GamePlayerActionResult<Option<(PlayerGameRequest, TakTimeInfo)>> {
        self.game_player_action(
            game_id,
            player,
            |game_entry, current_player| match game_entry.game.check_timeout(now) {
                MaybeTimeout::Timeout(game) => Ok(MaybeTimeout::Timeout(game)),
                MaybeTimeout::Result(()) => Ok(MaybeTimeout::Result(
                    match game_entry
                        .requests
                        .consume_request(current_player.opponent(), GameRequestType::MoreTime)
                    {
                        GameRequest::MoreTime(Some(duration)) => {
                            match game_entry
                                .game
                                .give_time_to_player(current_player, duration, now)
                            {
                                MaybeTimeout::Timeout(finished_game) => {
                                    return Ok(MaybeTimeout::Timeout(finished_game));
                                }
                                MaybeTimeout::Result(()) => Some(()),
                            }
                        }
                        _ => None,
                    },
                )),
            },
            |game_entry, current_player, res| match res {
                Some(()) => {
                    let time_info = game_entry.game.get_time_info(now);
                    let opponent_id = game_entry.metadata.get_player_id(current_player.opponent());
                    (
                        GameControl::Keep,
                        Some((
                            PlayerGameRequest {
                                player_id: opponent_id,
                                request: GameRequest::MoreTime(None),
                            },
                            time_info,
                        )),
                    )
                }
                None => (GameControl::Keep, None),
            },
        )
    }

    fn check_timeout(&self, game_id: GameId, now: Instant) -> CheckTimeoutResult {
        self.with_game_might_end(game_id, |game_entry| {
            match game_entry.game.check_timeout(now) {
                MaybeTimeout::Timeout(finished_game) => {
                    let time_info = finished_game.get_time_info();
                    game_entry.events.push(GameEvent::new(
                        GameEventType::GameOver(GameOverEventType::Timeout),
                        time_info,
                    ));
                    let finished_game = FinishedGame::new(game_entry, finished_game);
                    (
                        GameControl::Remove,
                        CheckTimeoutResult::TimedOut(finished_game),
                    )
                }
                MaybeTimeout::Result(()) => {
                    let time_info = game_entry.game.get_time_info(now);
                    (GameControl::Keep, CheckTimeoutResult::NoTimeout(time_info))
                }
            }
        })
        .unwrap_or(CheckTimeoutResult::GameNotFound)
    }

    fn check_disconnect_timeout(
        &self,
        game_id: GameId,
        player: PlayerId,
        disconnected_duration: Duration,
        now: Instant,
    ) -> GamePlayerActionResult<CheckDisconnectTimeoutResult> {
        self.game_player_action(
            game_id,
            player,
            |game_entry, current_player| {
                if matches!(
                    game_entry.metadata.settings.time_settings,
                    TakTimeSettings::Async(_)
                ) {
                    return Err(CheckDisconnectTimeoutResult::CantTimeOut);
                }
                let timeout_duration = Duration::from_secs(60 * 5);
                if disconnected_duration < timeout_duration {
                    return Err(CheckDisconnectTimeoutResult::NoTimeout(
                        timeout_duration - disconnected_duration,
                    ));
                }
                Ok(game_entry.game.resign_or_abandon(current_player, now))
            },
            |game_entry, _, finished_game| {
                let time_info = finished_game.get_time_info();
                game_entry.events.push(GameEvent::new(
                    GameEventType::GameOver(GameOverEventType::Abandonment),
                    time_info,
                ));
                let finished_game = FinishedGame::new(game_entry, finished_game);
                (
                    GameControl::Remove,
                    CheckDisconnectTimeoutResult::TimedOut(finished_game),
                )
            },
        )
    }
}
