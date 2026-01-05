use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use crate::domain::{GameId, GameType, PlayerId};
use dashmap::DashMap;
use tak_core::{
    MaybeTimeout, TakAction, TakActionRecord, TakFinishedGame, TakGame, TakGameSettings,
    TakOngoingGame, TakPlayer,
};

#[derive(Clone, Debug)]
pub struct GameMetadata {
    pub game_id: GameId,
    pub date: chrono::DateTime<chrono::Utc>,
    pub white_id: PlayerId,
    pub black_id: PlayerId,
    pub settings: TakGameSettings,
    pub game_type: GameType,
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
}

#[derive(Clone, Debug)]
pub struct FinishedGame {
    pub metadata: GameMetadata,
    pub game: TakFinishedGame,
}

impl FinishedGame {
    pub fn new(game: &OngoingGame, tak_game: TakFinishedGame) -> Self {
        Self {
            metadata: game.metadata.clone(),
            game: tak_game,
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
        game_type: GameType,
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

pub enum DoActionResult {
    ActionPerformed(TakActionRecord),
    GameOver(TakActionRecord, FinishedGame),
    Timeout(FinishedGame),
    GameNotFound,
    NotPlayersTurn,
    NotAPlayerInGame,
    InvalidAction(tak_core::DoActionError),
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
    Unchanged,
    Changed(TakOngoingGame),
    Ended,
}

impl GameServiceImpl {
    pub fn new() -> Self {
        Self {
            games: Arc::new(DashMap::new()),
        }
    }

    fn with_game_might_end<F, R>(&self, game_id: GameId, f: F) -> Option<R>
    where
        F: FnOnce(&OngoingGame) -> (GameControl, R),
    {
        let mut res = None;
        self.games.remove_if_mut(&game_id, |_, entry| {
            let (new_game, r) = f(entry);
            res = Some(r);
            match new_game {
                GameControl::Changed(g) => {
                    entry.game = g;
                    false
                }
                GameControl::Ended => true,
                GameControl::Unchanged => false,
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
        game_type: GameType,
        game_settings: TakGameSettings,
    ) -> OngoingGame {
        let game = TakOngoingGame::new(game_settings.clone());
        let metadata = GameMetadata {
            game_id: id,
            date,
            white_id,
            black_id,
            settings: game_settings.clone(),
            game_type,
        };
        let game_struct = OngoingGame { game, metadata };
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
                None => return (GameControl::Unchanged, DoActionResult::NotAPlayerInGame),
            };

            if game_entry.game.current_player() != current_player {
                return (GameControl::Unchanged, DoActionResult::NotPlayersTurn);
            }

            match game_entry.game.do_action(&action, now) {
                Ok(MaybeTimeout::Timeout(finished_game)) => {
                    let finished_game = FinishedGame::new(game_entry, finished_game);
                    (GameControl::Ended, DoActionResult::Timeout(finished_game))
                }
                Ok(MaybeTimeout::Result((action, game))) => match game {
                    TakGame::Finished(finished_game) => {
                        let finished_game = FinishedGame::new(game_entry, finished_game);
                        (
                            GameControl::Ended,
                            DoActionResult::GameOver(action, finished_game),
                        )
                    }
                    TakGame::Ongoing(ongoing_game) => (
                        GameControl::Changed(ongoing_game),
                        DoActionResult::ActionPerformed(action),
                    ),
                },
                Err(e) => (GameControl::Unchanged, DoActionResult::InvalidAction(e)),
            }
        })
        .unwrap_or(DoActionResult::GameNotFound)
    }

    fn resign(&self, game_id: GameId, player: PlayerId, now: Instant) -> ResignResult {
        self.with_game_might_end(game_id, |game_entry| {
            let current_player = match game_entry.metadata.get_player(player) {
                Some(p) => p,
                None => return (GameControl::Unchanged, ResignResult::NotAPlayerInGame),
            };

            match game_entry.game.resign(&current_player, now) {
                MaybeTimeout::Timeout(finished_game) => {
                    let finished_game = FinishedGame::new(game_entry, finished_game);
                    (GameControl::Ended, ResignResult::Timeout(finished_game))
                }
                MaybeTimeout::Result(finished_game) => {
                    let finished_game = FinishedGame::new(game_entry, finished_game);
                    (GameControl::Ended, ResignResult::GameOver(finished_game))
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
                None => return (GameControl::Unchanged, OfferDrawResult::NotAPlayerInGame),
            };

            match game_entry
                .game
                .offer_draw(&current_player, offer_status, now)
            {
                MaybeTimeout::Timeout(finished_game) => {
                    let finished_game = FinishedGame::new(game_entry, finished_game);
                    (GameControl::Ended, OfferDrawResult::Timeout(finished_game))
                }
                MaybeTimeout::Result(Some(TakGame::Finished(drawn_game))) => {
                    let finished_game = FinishedGame::new(game_entry, drawn_game);
                    (
                        GameControl::Ended,
                        OfferDrawResult::GameDrawn(finished_game),
                    )
                }
                MaybeTimeout::Result(Some(TakGame::Ongoing(ongoing_game))) => {
                    (GameControl::Changed(ongoing_game), OfferDrawResult::Success)
                }
                MaybeTimeout::Result(None) => (GameControl::Unchanged, OfferDrawResult::Unchanged),
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
                None => return (GameControl::Unchanged, RequestUndoResult::NotAPlayerInGame),
            };

            match game_entry
                .game
                .request_undo(&current_player, offer_status, now)
            {
                MaybeTimeout::Timeout(finished_game) => {
                    let finished_game = FinishedGame::new(game_entry, finished_game);
                    (
                        GameControl::Ended,
                        RequestUndoResult::Timeout(finished_game),
                    )
                }
                MaybeTimeout::Result(Some(ongoing_game)) => (
                    GameControl::Changed(ongoing_game),
                    RequestUndoResult::Success,
                ),
                MaybeTimeout::Result(None) => {
                    (GameControl::Unchanged, RequestUndoResult::Unchanged)
                }
            }
        })
        .unwrap_or(RequestUndoResult::GameNotFound)
    }

    fn check_timeout(&self, game_id: GameId, now: Instant) -> CheckTimoutResult {
        self.with_game_might_end(game_id, |game_entry| {
            match game_entry.game.check_timeout(now) {
                Some(finished_game) => {
                    let finished_game = FinishedGame::new(game_entry, finished_game);
                    (
                        GameControl::Ended,
                        CheckTimoutResult::GameTimedOut(finished_game),
                    )
                }
                None => {
                    let (white_remaining, black_remaining) =
                        game_entry.game.get_time_remaining_both(now);
                    (
                        GameControl::Unchanged,
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
