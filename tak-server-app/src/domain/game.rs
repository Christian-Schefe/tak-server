use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use crate::domain::{GameId, GameType, PlayerId};
use dashmap::DashMap;
use tak_core::{TakAction, TakActionRecord, TakGame, TakGameSettings, TakPlayer};

#[derive(Clone, Debug)]
pub struct Game {
    pub game_id: GameId,
    pub date: chrono::DateTime<chrono::Utc>,
    pub white_id: PlayerId,
    pub black_id: PlayerId,
    pub game: TakGame,
    pub settings: TakGameSettings,
    pub game_type: GameType,
}

impl Game {
    pub fn get_opponent(&self, player: PlayerId) -> Option<PlayerId> {
        if player == self.white_id {
            Some(self.black_id)
        } else if player == self.black_id {
            Some(self.white_id)
        } else {
            None
        }
    }

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
    ) -> Game;
    fn get_game_by_id(&self, game_id: GameId) -> Option<Game>;
    fn get_games(&self) -> Vec<Game>;
    fn do_action(
        &self,
        game_id: GameId,
        player: PlayerId,
        action: TakAction,
        now: Instant,
    ) -> Result<DoActionSuccess, DoActionError>;
    fn resign(&self, game_id: GameId, player: PlayerId, now: Instant) -> Result<Game, ResignError>;
    fn request_undo(
        &self,
        game_id: GameId,
        player: PlayerId,
        now: Instant,
    ) -> Result<RequestUndoSuccess, RequestUndoError>;
    fn retract_undo_request(
        &self,
        game_id: GameId,
        player: PlayerId,
        now: Instant,
    ) -> Result<bool, RequestUndoError>;
    fn offer_draw(
        &self,
        game_id: GameId,
        player: PlayerId,
        now: Instant,
    ) -> Result<OfferDrawSuccess, OfferDrawError>;
    fn retract_draw_offer(
        &self,
        game_id: GameId,
        player: PlayerId,
        now: Instant,
    ) -> Result<bool, OfferDrawError>;
    fn check_timeout(&self, game_id: GameId, now: Instant) -> CheckTimoutResult;
}

pub enum DoActionError {
    GameNotFound,
    NotPlayersTurn,
    InvalidAction,
}

pub struct TimeRemaining {
    pub white_time: Duration,
    pub black_time: Duration,
}

pub enum DoActionSuccess {
    ActionPerformed(TakActionRecord),
    GameOver(TakActionRecord, Game),
}

pub enum ResignError {
    GameNotFound,
    NotAPlayerInGame,
    InvalidResign,
}

pub enum OfferDrawError {
    GameNotFound,
    NotAPlayerInGame,
    InvalidOffer,
}

pub enum OfferDrawSuccess {
    DrawOffered(bool),
    GameDrawn(Game),
}
pub enum RequestUndoSuccess {
    UndoRequested(bool),
    MoveUndone,
}

pub enum RequestUndoError {
    GameNotFound,
    NotAPlayerInGame,
    InvalidRequest,
}

pub enum CheckTimoutResult {
    GameTimedOut(Game),
    NoTimeout {
        white_remaining: Duration,
        black_remaining: Duration,
    },
}

pub struct GameServiceImpl {
    games: Arc<DashMap<GameId, Game>>,
}

impl GameServiceImpl {
    pub fn new() -> Self {
        Self {
            games: Arc::new(DashMap::new()),
        }
    }

    fn handle_maybe_game_over(&self, game_id: GameId) -> Option<Game> {
        let (_, game_entry) = self
            .games
            .remove_if(&game_id, |_, game| !game.game.is_ongoing())?;
        Some(game_entry)
    }

    fn with_game<F, R>(&self, game_id: GameId, f: F) -> Option<R>
    where
        F: FnOnce(&mut Game) -> R,
    {
        self.games.get_mut(&game_id).map(|mut entry| f(&mut entry))
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
    ) -> Game {
        let game = TakGame::new(game_settings.clone());
        let game_struct = Game {
            date,
            white_id,
            black_id,
            game,
            game_type,
            settings: game_settings,
            game_id: id,
        };
        self.games.insert(id, game_struct.clone());

        game_struct
    }

    fn get_game_by_id(&self, game_id: GameId) -> Option<Game> {
        self.games.get(&game_id).map(|entry| entry.clone())
    }

    fn get_games(&self) -> Vec<Game> {
        self.games.iter().map(|entry| entry.clone()).collect()
    }

    fn do_action(
        &self,
        game_id: GameId,
        player: PlayerId,
        action: TakAction,
        now: Instant,
    ) -> Result<DoActionSuccess, DoActionError> {
        let game_record = self
            .with_game(game_id, |game_entry| {
                let current_player = match player {
                    id if id == game_entry.white_id => TakPlayer::White,
                    id if id == game_entry.black_id => TakPlayer::Black,
                    _ => return Err(DoActionError::NotPlayersTurn),
                };

                if game_entry.game.current_player() != current_player {
                    return Err(DoActionError::NotPlayersTurn);
                }

                match game_entry.game.do_action(&action, now) {
                    Ok(record) => Ok(record),
                    Err(_) => Err(DoActionError::InvalidAction),
                }
            })
            .unwrap_or(Err(DoActionError::GameNotFound))?;

        if let Some(ended_game) = self.handle_maybe_game_over(game_id) {
            return Ok(DoActionSuccess::GameOver(game_record, ended_game));
        };

        Ok(DoActionSuccess::ActionPerformed(game_record))
    }

    fn resign(&self, game_id: GameId, player: PlayerId, now: Instant) -> Result<Game, ResignError> {
        self.with_game(game_id, |game_entry| {
            let current_player = match player {
                id if id == game_entry.white_id => TakPlayer::White,
                id if id == game_entry.black_id => TakPlayer::Black,
                _ => return Err(ResignError::NotAPlayerInGame),
            };

            if let Err(_) = game_entry.game.resign(&current_player, now) {
                return Err(ResignError::InvalidResign);
            }
            Ok(())
        })
        .unwrap_or(Err(ResignError::GameNotFound))?;

        let Some(ended_game) = self.handle_maybe_game_over(game_id) else {
            return Err(ResignError::GameNotFound);
        };
        Ok(ended_game)
    }

    fn offer_draw(
        &self,
        game_id: GameId,
        player: PlayerId,
        now: Instant,
    ) -> Result<OfferDrawSuccess, OfferDrawError> {
        let (did_draw, changed) = self
            .with_game(game_id, |game_entry| {
                let current_player = match player {
                    id if id == game_entry.white_id => TakPlayer::White,
                    id if id == game_entry.black_id => TakPlayer::Black,
                    _ => return Err(OfferDrawError::NotAPlayerInGame),
                };

                let changed = !game_entry.game.get_draw_offer(&current_player);

                match game_entry.game.offer_draw(&current_player, true, now) {
                    Ok(did_draw) => Ok((did_draw, changed)),
                    Err(_) => Err(OfferDrawError::InvalidOffer),
                }
            })
            .unwrap_or(Err(OfferDrawError::GameNotFound))?;
        if did_draw {
            let Some(ended_game) = self.handle_maybe_game_over(game_id) else {
                return Err(OfferDrawError::GameNotFound);
            };
            Ok(OfferDrawSuccess::GameDrawn(ended_game))
        } else {
            Ok(OfferDrawSuccess::DrawOffered(changed))
        }
    }

    fn retract_draw_offer(
        &self,
        game_id: GameId,
        player: PlayerId,
        now: Instant,
    ) -> Result<bool, OfferDrawError> {
        let changed = self
            .with_game(game_id, |game_entry| {
                let current_player = match player {
                    id if id == game_entry.white_id => TakPlayer::White,
                    id if id == game_entry.black_id => TakPlayer::Black,
                    _ => return Err(OfferDrawError::NotAPlayerInGame),
                };

                let changed = game_entry.game.get_draw_offer(&current_player);

                match game_entry.game.offer_draw(&current_player, false, now) {
                    Ok(_) => Ok(changed),
                    Err(_) => Err(OfferDrawError::InvalidOffer),
                }
            })
            .unwrap_or(Err(OfferDrawError::GameNotFound))?;
        Ok(changed)
    }

    fn request_undo(
        &self,
        game_id: GameId,
        player: PlayerId,
        now: Instant,
    ) -> Result<RequestUndoSuccess, RequestUndoError> {
        let result = self
            .with_game(game_id, |game_entry| {
                let current_player = match player {
                    id if id == game_entry.white_id => TakPlayer::White,
                    id if id == game_entry.black_id => TakPlayer::Black,
                    _ => return Err(RequestUndoError::NotAPlayerInGame),
                };

                let changed = !game_entry.game.get_undo_request(&current_player);

                match game_entry.game.request_undo(&current_player, true, now) {
                    Ok(false) => Ok(RequestUndoSuccess::UndoRequested(changed)),
                    Ok(true) => Ok(RequestUndoSuccess::MoveUndone),
                    Err(_) => return Err(RequestUndoError::InvalidRequest),
                }
            })
            .unwrap_or(Err(RequestUndoError::GameNotFound))?;

        self.handle_maybe_game_over(game_id);

        Ok(result)
    }

    fn retract_undo_request(
        &self,
        game_id: GameId,
        player: PlayerId,
        now: Instant,
    ) -> Result<bool, RequestUndoError> {
        let changed = self
            .with_game(game_id, |game_entry| {
                let current_player = match player {
                    id if id == game_entry.white_id => TakPlayer::White,
                    id if id == game_entry.black_id => TakPlayer::Black,
                    _ => return Err(RequestUndoError::NotAPlayerInGame),
                };

                let changed = game_entry.game.get_undo_request(&current_player);

                match game_entry.game.request_undo(&current_player, false, now) {
                    Ok(_) => Ok(changed),
                    Err(_) => Err(RequestUndoError::InvalidRequest),
                }
            })
            .unwrap_or(Err(RequestUndoError::GameNotFound))?;
        Ok(changed)
    }

    fn check_timeout(&self, game_id: GameId, now: Instant) -> CheckTimoutResult {
        let mut times_remaining = None;
        let Some((_, ended_game)) = self.games.remove_if_mut(&game_id, |_, game_ref| {
            game_ref.game.check_timeout(now);
            times_remaining = Some(game_ref.game.get_time_remaining_both(now));
            !game_ref.game.is_ongoing()
        }) else {
            let (white_remaining, black_remaining) =
                times_remaining.unwrap_or((Duration::ZERO, Duration::ZERO));
            return CheckTimoutResult::NoTimeout {
                white_remaining,
                black_remaining,
            };
        };
        CheckTimoutResult::GameTimedOut(ended_game)
    }
}
