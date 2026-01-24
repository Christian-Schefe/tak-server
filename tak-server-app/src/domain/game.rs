use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use crate::domain::{
    GameId, PlayerId,
    game::request::{GameRequest, GameRequestId, GameRequestSystem, GameRequestType},
};
use dashmap::DashMap;
use tak_core::{
    MaybeTimeout, TakAction, TakFinishedRealtimeGame, TakOngoingRealtimeGame, TakPlayer,
    TakRealtimeGameSettings,
};

pub mod request;

#[derive(Clone, Debug)]
pub struct GameEvent {
    pub event_type: GameEventType,
    pub date: chrono::DateTime<chrono::Utc>,
}

impl GameEvent {
    pub fn new(event_type: GameEventType) -> Self {
        Self {
            event_type,
            date: chrono::Utc::now(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum GameEventType {
    Action {
        action: TakAction,
        white_remaining: Duration,
        black_remaining: Duration,
    },
    RequestAdded {
        request: GameRequest,
    },
    RequestRetracted {
        request_id: GameRequestId,
    },
    RequestRejected {
        request_id: GameRequestId,
    },
    RequestAccepted {
        request_id: GameRequestId,
    },
    ActionUndone,
    DrawAgreed,
    TimeGiven {
        player: TakPlayer,
        duration: Duration,
    },
    Timeout,
    Resigned(TakPlayer),
}

#[derive(Clone, Debug)]
pub struct GameMetadata {
    pub game_id: GameId,
    pub date: chrono::DateTime<chrono::Utc>,
    pub white_id: PlayerId,
    pub black_id: PlayerId,
    pub settings: TakRealtimeGameSettings,
    pub is_rated: bool,
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
pub struct OngoingGame {
    pub metadata: GameMetadata,
    pub game: TakOngoingRealtimeGame,
    pub requests: GameRequestSystem,
    pub events: Vec<GameEvent>,
}

#[derive(Clone, Debug)]
pub struct FinishedGame {
    pub metadata: GameMetadata,
    pub game: TakFinishedRealtimeGame,
    pub events: Vec<GameEvent>,
}

impl FinishedGame {
    pub fn new(game: &OngoingGame, tak_game: TakFinishedRealtimeGame) -> Self {
        Self {
            metadata: game.metadata.clone(),
            game: tak_game,
            events: game.events.clone(),
        }
    }
    pub fn get_time_remaining(&self) -> TimeRemaining {
        let (white_time, black_time) = self.game.get_time_remaining();
        TimeRemaining {
            white_time,
            black_time,
        }
    }
}

impl OngoingGame {
    pub fn get_time_remaining(&self, now: Instant) -> TimeRemaining {
        let (white_time, black_time) = self.game.get_time_remaining_both(now);
        TimeRemaining {
            white_time,
            black_time,
        }
    }
}

pub trait GameService {
    fn create_game(
        &self,
        id: GameId,
        date: chrono::DateTime<chrono::Utc>,
        white_id: PlayerId,
        black_id: PlayerId,
        is_rated: bool,
        game_settings: TakRealtimeGameSettings,
    ) -> OngoingGame;
    fn get_game_by_id(&self, game_id: GameId) -> Option<OngoingGame>;
    fn get_games(&self) -> Vec<OngoingGame>;
    fn check_timeout(&self, game_id: GameId, now: Instant) -> CheckTimoutResult;
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
    ) -> GamePlayerActionResult<ResignResult>;
    fn add_request(
        &self,
        game_id: GameId,
        player: PlayerId,
        request: GameRequestType,
        now: Instant,
    ) -> GamePlayerActionResult<Result<GameRequest, ()>>;
    fn retract_request(
        &self,
        game_id: GameId,
        player: PlayerId,
        request_id: GameRequestId,
        now: Instant,
    ) -> GamePlayerActionResult<Result<GameRequest, ()>>;
    fn reject_request(
        &self,
        game_id: GameId,
        player: PlayerId,
        request_id: GameRequestId,
        now: Instant,
    ) -> GamePlayerActionResult<Result<GameRequest, ()>>;

    fn accept_draw_request(
        &self,
        game_id: GameId,
        player: PlayerId,
        request_id: GameRequestId,
        now: Instant,
    ) -> GamePlayerActionResult<Result<(GameRequest, FinishedGame), ()>>;
    fn accept_undo_request(
        &self,
        game_id: GameId,
        player: PlayerId,
        request_id: GameRequestId,
        now: Instant,
    ) -> GamePlayerActionResult<Result<(GameRequest, bool), ()>>;
}

#[derive(Clone, Debug)]
pub struct GameActionRecord {
    pub action: TakAction,
    pub ply_index: usize,
}

impl GameActionRecord {
    pub fn new(action: TakAction, ply_index: usize) -> Self {
        Self { action, ply_index }
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

pub enum ResignResult {
    GameOver(FinishedGame),
}

pub struct TimeRemaining {
    pub white_time: Duration,
    pub black_time: Duration,
}

pub enum CheckTimoutResult {
    GameNotFound,
    GameTimedOut(FinishedGame),
    NoTimeout(TimeRemaining),
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
        ) -> Result<MaybeTimeout<FR, TakFinishedRealtimeGame>, R>,
        decision_fn: impl FnOnce(&mut OngoingGame, TakPlayer, FR) -> (GameControl, R),
    ) -> GamePlayerActionResult<R> {
        self.with_game_might_end(game_id, |game_entry| {
            let current_player = match get_current_player(game_entry, player) {
                Ok(p) => p,
                Err(e) => return e,
            };
            let action_res = action_fn(game_entry, current_player);
            match action_res {
                Ok(MaybeTimeout::Timeout(finished_game)) => on_timeout(game_entry, finished_game),
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
    fn create_game(
        &self,
        id: GameId,
        date: chrono::DateTime<chrono::Utc>,
        white_id: PlayerId,
        black_id: PlayerId,
        is_rated: bool,
        game_settings: TakRealtimeGameSettings,
    ) -> OngoingGame {
        let game = TakOngoingRealtimeGame::new(game_settings.clone());
        let metadata = GameMetadata {
            game_id: id,
            date,
            white_id,
            black_id,
            settings: game_settings.clone(),
            is_rated,
        };
        let game_struct = OngoingGame {
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

    fn get_games(&self) -> Vec<OngoingGame> {
        self.games.iter().map(|entry| entry.clone()).collect()
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
            |game_entry, _, res| {
                let ply_index = game_entry.game.action_history().len() - 1;
                match res {
                    Some(finished_game) => {
                        let (white_remaining, black_remaining) = finished_game.get_time_remaining();
                        game_entry
                            .events
                            .push(GameEvent::new(GameEventType::Action {
                                action: action.clone(),
                                white_remaining,
                                black_remaining,
                            }));
                        let finished_game = FinishedGame::new(game_entry, finished_game);
                        (
                            GameControl::Remove,
                            DoActionResult::GameOver(
                                GameActionRecord::new(action.clone(), ply_index),
                                finished_game,
                            ),
                        )
                    }
                    None => {
                        let (white_remaining, black_remaining) =
                            game_entry.game.get_time_remaining_both(now);
                        game_entry
                            .events
                            .push(GameEvent::new(GameEventType::Action {
                                action: action.clone(),
                                white_remaining,
                                black_remaining,
                            }));
                        (
                            GameControl::Keep,
                            DoActionResult::ActionPerformed(GameActionRecord::new(
                                action.clone(),
                                ply_index,
                            )),
                        )
                    }
                }
            },
        )
    }

    fn resign(
        &self,
        game_id: GameId,
        player: PlayerId,
        now: Instant,
    ) -> GamePlayerActionResult<ResignResult> {
        self.game_player_action(
            game_id,
            player,
            |game_entry, current_player| Ok(game_entry.game.resign(&current_player, now)),
            |game_entry, current_player, finished_game| {
                game_entry
                    .events
                    .push(GameEvent::new(GameEventType::Resigned(current_player)));
                let finished_game = FinishedGame::new(game_entry, finished_game);
                (GameControl::Remove, ResignResult::GameOver(finished_game))
            },
        )
    }

    fn add_request(
        &self,
        game_id: GameId,
        player: PlayerId,
        request_type: GameRequestType,
        now: Instant,
    ) -> GamePlayerActionResult<Result<GameRequest, ()>> {
        self.game_player_action(
            game_id,
            player,
            |game_entry, current_player| match game_entry.game.check_timeout(now) {
                MaybeTimeout::Timeout(game) => Ok(MaybeTimeout::Timeout(game)),
                MaybeTimeout::Result(()) => Ok(MaybeTimeout::Result(
                    game_entry
                        .requests
                        .add_request(&current_player, request_type),
                )),
            },
            |game_entry, _, res| match res {
                Some(request) => {
                    game_entry
                        .events
                        .push(GameEvent::new(GameEventType::RequestAdded {
                            request: request.clone(),
                        }));
                    (GameControl::Keep, Ok(request))
                }
                None => (GameControl::Keep, Err(())),
            },
        )
    }

    fn retract_request(
        &self,
        game_id: GameId,
        player: PlayerId,
        request_id: GameRequestId,
        now: Instant,
    ) -> GamePlayerActionResult<Result<GameRequest, ()>> {
        self.game_player_action(
            game_id,
            player,
            |game_entry, current_player| match game_entry.game.check_timeout(now) {
                MaybeTimeout::Timeout(game) => Ok(MaybeTimeout::Timeout(game)),
                MaybeTimeout::Result(()) => Ok(MaybeTimeout::Result(
                    game_entry
                        .requests
                        .take_request_if(request_id, |p| p.player == current_player),
                )),
            },
            |game_entry, _, res| match res {
                Some(request) => {
                    game_entry
                        .events
                        .push(GameEvent::new(GameEventType::RequestRetracted {
                            request_id,
                        }));
                    (GameControl::Keep, Ok(request))
                }
                None => (GameControl::Keep, Err(())),
            },
        )
    }
    fn reject_request(
        &self,
        game_id: GameId,
        player: PlayerId,
        request_id: GameRequestId,
        now: Instant,
    ) -> GamePlayerActionResult<Result<GameRequest, ()>> {
        self.game_player_action(
            game_id,
            player,
            |game_entry, current_player| match game_entry.game.check_timeout(now) {
                MaybeTimeout::Timeout(game) => Ok(MaybeTimeout::Timeout(game)),
                MaybeTimeout::Result(()) => Ok(MaybeTimeout::Result(
                    game_entry
                        .requests
                        .take_request_if(request_id, |p| p.player != current_player),
                )),
            },
            |game_entry, _, res| match res {
                Some(request) => {
                    game_entry
                        .events
                        .push(GameEvent::new(GameEventType::RequestRejected {
                            request_id,
                        }));
                    (GameControl::Keep, Ok(request))
                }
                None => (GameControl::Keep, Err(())),
            },
        )
    }
    fn accept_draw_request(
        &self,
        game_id: GameId,
        player: PlayerId,
        request_id: GameRequestId,
        now: Instant,
    ) -> GamePlayerActionResult<Result<(GameRequest, FinishedGame), ()>> {
        self.game_player_action(
            game_id,
            player,
            |game_entry, current_player| match game_entry.game.check_timeout(now) {
                MaybeTimeout::Timeout(game) => Ok(MaybeTimeout::Timeout(game)),
                MaybeTimeout::Result(()) => Ok(MaybeTimeout::Result(
                    if let Some(request) =
                        game_entry.requests.take_request_if(request_id, |request| {
                            request.player != current_player
                                && matches!(request.request_type, GameRequestType::Draw)
                        })
                    {
                        match game_entry.game.agree_draw(now) {
                            MaybeTimeout::Timeout(finished_game) => {
                                return Ok(MaybeTimeout::Timeout(finished_game));
                            }
                            MaybeTimeout::Result(finished_game) => Some((request, finished_game)),
                        }
                    } else {
                        None
                    },
                )),
            },
            |game_entry, _, res| match res {
                Some((request, finished_game)) => {
                    game_entry
                        .events
                        .push(GameEvent::new(GameEventType::RequestAccepted {
                            request_id,
                        }));
                    game_entry
                        .events
                        .push(GameEvent::new(GameEventType::DrawAgreed));
                    let finished_game = FinishedGame::new(game_entry, finished_game);
                    (GameControl::Remove, Ok((request, finished_game)))
                }
                None => (GameControl::Keep, Err(())),
            },
        )
    }
    fn accept_undo_request(
        &self,
        game_id: GameId,
        player: PlayerId,
        request_id: GameRequestId,
        now: Instant,
    ) -> GamePlayerActionResult<Result<(GameRequest, bool), ()>> {
        self.game_player_action(
            game_id,
            player,
            |game_entry, current_player| match game_entry.game.check_timeout(now) {
                MaybeTimeout::Timeout(game) => Ok(MaybeTimeout::Timeout(game)),
                MaybeTimeout::Result(()) => Ok(MaybeTimeout::Result(
                    if let Some(request) =
                        game_entry.requests.take_request_if(request_id, |request| {
                            request.player != current_player
                                && matches!(request.request_type, GameRequestType::Draw)
                        })
                    {
                        match game_entry.game.undo_action(now) {
                            MaybeTimeout::Timeout(finished_game) => {
                                return Ok(MaybeTimeout::Timeout(finished_game));
                            }
                            MaybeTimeout::Result(did_undo) => Some((request, did_undo)),
                        }
                    } else {
                        None
                    },
                )),
            },
            |game_entry, _, res| match res {
                Some((request, did_undo)) => {
                    game_entry
                        .events
                        .push(GameEvent::new(GameEventType::RequestAccepted {
                            request_id,
                        }));
                    if did_undo {
                        game_entry
                            .events
                            .push(GameEvent::new(GameEventType::ActionUndone));
                    }
                    (GameControl::Keep, Ok((request, did_undo)))
                }
                None => (GameControl::Keep, Err(())),
            },
        )
    }

    fn check_timeout(&self, game_id: GameId, now: Instant) -> CheckTimoutResult {
        self.with_game_might_end(game_id, |game_entry| {
            match game_entry.game.check_timeout(now) {
                MaybeTimeout::Timeout(finished_game) => {
                    let finished_game = FinishedGame::new(game_entry, finished_game);
                    game_entry
                        .events
                        .push(GameEvent::new(GameEventType::Timeout));
                    (
                        GameControl::Remove,
                        CheckTimoutResult::GameTimedOut(finished_game),
                    )
                }
                MaybeTimeout::Result(()) => {
                    let (white_remaining, black_remaining) =
                        game_entry.game.get_time_remaining_both(now);
                    (
                        GameControl::Keep,
                        CheckTimoutResult::NoTimeout(TimeRemaining {
                            white_time: white_remaining,
                            black_time: black_remaining,
                        }),
                    )
                }
            }
        })
        .unwrap_or(CheckTimoutResult::GameNotFound)
    }
}

fn get_current_player<R>(
    game_entry: &OngoingGame,
    player: PlayerId,
) -> Result<TakPlayer, (GameControl, GamePlayerActionResult<R>)> {
    match game_entry.metadata.get_player(player) {
        Some(p) => Ok(p),
        None => Err((GameControl::Keep, GamePlayerActionResult::NotAPlayerInGame)),
    }
}

fn on_timeout<R>(
    game_entry: &mut OngoingGame,
    finished_game: TakFinishedRealtimeGame,
) -> (GameControl, GamePlayerActionResult<R>) {
    game_entry
        .events
        .push(GameEvent::new(GameEventType::Timeout));
    let finished_game = FinishedGame::new(game_entry, finished_game);
    (
        GameControl::Remove,
        GamePlayerActionResult::Timeout(finished_game),
    )
}
