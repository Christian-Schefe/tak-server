use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use crate::domain::{GameId, PlayerId};
use dashmap::DashMap;
use tak_core::{
    MaybeTimeout, TakAction, TakFinishedGame, TakGame, TakGameSettings, TakOngoingGame, TakPlayer,
};

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
    UndoRequested(TakPlayer),
    UndoRequestWithdrawn(TakPlayer),
    UndoAccepted,
    DrawOffered(TakPlayer),
    DrawOfferWithdrawn(TakPlayer),
    DrawAgreed,
    Timeout,
    Resigned(TakPlayer),
}

#[derive(Clone, Debug)]
pub struct GameMetadata {
    pub game_id: GameId,
    pub date: chrono::DateTime<chrono::Utc>,
    pub white_id: PlayerId,
    pub black_id: PlayerId,
    pub settings: TakGameSettings,
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

    fn get_player(&self, id: PlayerId) -> Option<TakPlayer> {
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
    pub game: TakOngoingGame,
    pub events: Vec<GameEvent>,
}

#[derive(Clone, Debug)]
pub struct FinishedGame {
    pub metadata: GameMetadata,
    pub game: TakFinishedGame,
    pub events: Vec<GameEvent>,
}

impl FinishedGame {
    pub fn new(game: &OngoingGame, tak_game: TakFinishedGame) -> Self {
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
        game_settings: TakGameSettings,
    ) -> OngoingGame;
    fn get_game_by_id(&self, game_id: GameId) -> Option<OngoingGame>;
    fn get_games(&self) -> Vec<OngoingGame>;
    fn do_action(
        &self,
        game_id: GameId,
        player: PlayerId,
        action: TakAction,
        now: Instant,
    ) -> DoActionResult;
    fn resign(&self, game_id: GameId, player: PlayerId, now: Instant) -> ResignResult;
    fn request_undo(
        &self,
        game_id: GameId,
        player: PlayerId,
        now: Instant,
        offer_status: bool,
    ) -> RequestUndoResult;
    fn offer_draw(
        &self,
        game_id: GameId,
        player: PlayerId,
        now: Instant,
        offer_status: bool,
    ) -> OfferDrawResult;

    fn check_timeout(&self, game_id: GameId, now: Instant) -> CheckTimoutResult;
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
    Timeout(FinishedGame),
    GameNotFound,
    NotPlayersTurn,
    NotAPlayerInGame,
    InvalidAction(tak_core::InvalidActionReason),
}

pub enum ResignResult {
    GameNotFound,
    NotAPlayerInGame,
    Timeout(FinishedGame),
    GameOver(FinishedGame),
}

pub enum OfferDrawResult {
    Success,
    Unchanged,
    GameNotFound,
    NotAPlayerInGame,
    Timeout(FinishedGame),
    GameDrawn(FinishedGame),
}

pub enum RequestUndoResult {
    Success,
    Unchanged,
    GameNotFound,
    NotAPlayerInGame,
    Timeout(FinishedGame),
    MoveUndone,
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
}

impl GameService for GameServiceImpl {
    fn create_game(
        &self,
        id: GameId,
        date: chrono::DateTime<chrono::Utc>,
        white_id: PlayerId,
        black_id: PlayerId,
        is_rated: bool,
        game_settings: TakGameSettings,
    ) -> OngoingGame {
        let game = TakOngoingGame::new(game_settings.clone());
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
    ) -> DoActionResult {
        self.with_game_might_end(game_id, |game_entry| {
            let current_player = match game_entry.metadata.get_player(player) {
                Some(p) => p,
                None => {
                    return (GameControl::Keep, DoActionResult::NotAPlayerInGame);
                }
            };

            if game_entry.game.current_player() != current_player {
                return (GameControl::Keep, DoActionResult::NotPlayersTurn);
            }
            let ply_index = game_entry.game.action_history().len();
            match game_entry.game.do_action(action.clone(), now) {
                Ok(MaybeTimeout::Timeout(finished_game)) => {
                    game_entry
                        .events
                        .push(GameEvent::new(GameEventType::Timeout));
                    let finished_game = FinishedGame::new(game_entry, finished_game);
                    (GameControl::Remove, DoActionResult::Timeout(finished_game))
                }
                Ok(MaybeTimeout::Result(game)) => match game {
                    TakGame::Finished(finished_game) => {
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
                                GameActionRecord::new(action, ply_index),
                                finished_game,
                            ),
                        )
                    }
                    TakGame::Ongoing(ongoing_game) => {
                        let (white_remaining, black_remaining) =
                            ongoing_game.get_time_remaining_both(now);
                        game_entry
                            .events
                            .push(GameEvent::new(GameEventType::Action {
                                action: action.clone(),
                                white_remaining,
                                black_remaining,
                            }));
                        game_entry.game = ongoing_game;
                        (
                            GameControl::Keep,
                            DoActionResult::ActionPerformed(GameActionRecord::new(
                                action, ply_index,
                            )),
                        )
                    }
                },
                Err(e) => (GameControl::Keep, DoActionResult::InvalidAction(e)),
            }
        })
        .unwrap_or(DoActionResult::GameNotFound)
    }

    fn resign(&self, game_id: GameId, player: PlayerId, now: Instant) -> ResignResult {
        self.with_game_might_end(game_id, |game_entry| {
            let current_player = match game_entry.metadata.get_player(player) {
                Some(p) => p,
                None => return (GameControl::Keep, ResignResult::NotAPlayerInGame),
            };

            match game_entry.game.resign(&current_player, now) {
                MaybeTimeout::Timeout(finished_game) => {
                    game_entry
                        .events
                        .push(GameEvent::new(GameEventType::Timeout));
                    let finished_game = FinishedGame::new(game_entry, finished_game);
                    (GameControl::Remove, ResignResult::Timeout(finished_game))
                }
                MaybeTimeout::Result(finished_game) => {
                    game_entry
                        .events
                        .push(GameEvent::new(GameEventType::Resigned(current_player)));
                    let finished_game = FinishedGame::new(game_entry, finished_game);
                    (GameControl::Remove, ResignResult::GameOver(finished_game))
                }
            }
        })
        .unwrap_or(ResignResult::GameNotFound)
    }

    fn offer_draw(
        &self,
        game_id: GameId,
        player: PlayerId,
        now: Instant,
        offer_status: bool,
    ) -> OfferDrawResult {
        self.with_game_might_end(game_id, |game_entry| {
            let current_player = match game_entry.metadata.get_player(player) {
                Some(p) => p,
                None => return (GameControl::Keep, OfferDrawResult::NotAPlayerInGame),
            };

            match game_entry
                .game
                .offer_draw(&current_player, offer_status, now)
            {
                MaybeTimeout::Timeout(finished_game) => {
                    game_entry
                        .events
                        .push(GameEvent::new(GameEventType::Timeout));
                    let finished_game = FinishedGame::new(game_entry, finished_game);
                    (GameControl::Remove, OfferDrawResult::Timeout(finished_game))
                }
                MaybeTimeout::Result(Some(TakGame::Finished(drawn_game))) => {
                    game_entry
                        .events
                        .push(GameEvent::new(GameEventType::DrawAgreed));
                    let finished_game = FinishedGame::new(game_entry, drawn_game);
                    (
                        GameControl::Remove,
                        OfferDrawResult::GameDrawn(finished_game),
                    )
                }
                MaybeTimeout::Result(Some(TakGame::Ongoing(ongoing_game))) => {
                    let event = if offer_status {
                        GameEvent::new(GameEventType::DrawOffered(current_player))
                    } else {
                        GameEvent::new(GameEventType::DrawOfferWithdrawn(current_player))
                    };
                    game_entry.events.push(event);
                    game_entry.game = ongoing_game;
                    (GameControl::Keep, OfferDrawResult::Success)
                }
                MaybeTimeout::Result(None) => (GameControl::Keep, OfferDrawResult::Unchanged),
            }
        })
        .unwrap_or(OfferDrawResult::GameNotFound)
    }

    fn request_undo(
        &self,
        game_id: GameId,
        player: PlayerId,
        now: Instant,
        offer_status: bool,
    ) -> RequestUndoResult {
        self.with_game_might_end(game_id, |game_entry| {
            let current_player = match game_entry.metadata.get_player(player) {
                Some(p) => p,
                None => return (GameControl::Keep, RequestUndoResult::NotAPlayerInGame),
            };

            match game_entry
                .game
                .request_undo(&current_player, offer_status, now)
            {
                MaybeTimeout::Timeout(finished_game) => {
                    game_entry
                        .events
                        .push(GameEvent::new(GameEventType::Timeout));
                    let finished_game = FinishedGame::new(game_entry, finished_game);
                    (
                        GameControl::Remove,
                        RequestUndoResult::Timeout(finished_game),
                    )
                }
                MaybeTimeout::Result(Some(ongoing_game)) => {
                    let event = if offer_status {
                        GameEvent::new(GameEventType::UndoRequested(current_player))
                    } else {
                        GameEvent::new(GameEventType::UndoRequestWithdrawn(current_player))
                    };
                    game_entry.events.push(event);
                    game_entry.game = ongoing_game;
                    (GameControl::Keep, RequestUndoResult::Success)
                }
                MaybeTimeout::Result(None) => (GameControl::Keep, RequestUndoResult::Unchanged),
            }
        })
        .unwrap_or(RequestUndoResult::GameNotFound)
    }

    fn check_timeout(&self, game_id: GameId, now: Instant) -> CheckTimoutResult {
        self.with_game_might_end(game_id, |game_entry| {
            match game_entry.game.check_timeout(now) {
                Some(finished_game) => {
                    let finished_game = FinishedGame::new(game_entry, finished_game);
                    game_entry
                        .events
                        .push(GameEvent::new(GameEventType::Timeout));
                    (
                        GameControl::Remove,
                        CheckTimoutResult::GameTimedOut(finished_game),
                    )
                }
                None => {
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
