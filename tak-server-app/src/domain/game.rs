use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crate::domain::{GameId, GameType, MatchId, PlayerId};
use dashmap::DashMap;
use tak_core::{TakAction, TakActionRecord, TakGame, TakGameSettings, TakPlayer};

#[derive(Clone, Debug)]
pub struct Game {
    pub game_id: GameId,
    pub match_id: MatchId,
    pub date: chrono::DateTime<chrono::Utc>,
    pub white: PlayerId,
    pub black: PlayerId,
    pub game: TakGame,
    pub settings: TakGameSettings,
    pub game_type: GameType,
}

impl Game {
    pub fn get_opponent(&self, player: PlayerId) -> Option<PlayerId> {
        if player == self.white {
            Some(self.black)
        } else if player == self.black {
            Some(self.white)
        } else {
            None
        }
    }
}

pub trait GameService {
    fn create_game(
        &self,
        date: chrono::DateTime<chrono::Utc>,
        player1: PlayerId,
        player2: PlayerId,
        color: Option<TakPlayer>,
        game_type: GameType,
        game_settings: TakGameSettings,
        match_id: MatchId,
    ) -> Game;
    fn get_game_by_id(&self, game_id: GameId) -> Option<Game>;
    fn get_games(&self) -> Vec<Game>;
    fn do_action(
        &self,
        game_id: GameId,
        player: PlayerId,
        action: TakAction,
    ) -> Result<DoActionSuccess, DoActionError>;
    fn resign(&self, game_id: GameId, player: PlayerId) -> Result<Game, ResignError>;
    fn request_undo(&self, game_id: GameId, player: PlayerId) -> Result<bool, RequestUndoError>;
    fn offer_draw(
        &self,
        game_id: GameId,
        player: PlayerId,
    ) -> Result<OfferDrawSuccess, OfferDrawError>;
    fn check_timeout(&self, game_id: GameId, now: Instant) -> CheckTimoutResult;
}

pub enum DoActionError {
    GameNotFound,
    NotPlayersTurn,
    InvalidAction,
}

pub enum DoActionSuccess {
    ActionPerformed(TakActionRecord),
    GameOver(TakActionRecord, Game),
}

pub enum ResignError {
    GameNotFound,
    NotPlayersTurn,
    InvalidResign,
}

pub enum OfferDrawError {
    GameNotFound,
    NotAPlayerInGame,
    InvalidOffer,
}

pub enum OfferDrawSuccess {
    DrawOffered,
    GameDrawn(Game),
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
    next_game_id: Arc<Mutex<GameId>>,
}

impl GameServiceImpl {
    pub fn new() -> Self {
        Self {
            games: Arc::new(DashMap::new()),
            next_game_id: Arc::new(Mutex::new(GameId(0))),
        }
    }

    fn increment_game_id(&self) -> GameId {
        let mut id_lock = self.next_game_id.lock().unwrap();
        let game_id = *id_lock;
        id_lock.0 += 1;
        game_id
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
        date: chrono::DateTime<chrono::Utc>,
        player1: PlayerId,
        player2: PlayerId,
        color: Option<TakPlayer>,
        game_type: GameType,
        game_settings: TakGameSettings,
        match_id: MatchId,
    ) -> Game {
        let (white, black) = match color {
            Some(TakPlayer::White) => (player1, player2),
            Some(TakPlayer::Black) => (player2, player1),
            None => {
                if rand::random() {
                    (player1, player2)
                } else {
                    (player2, player1)
                }
            }
        };
        let game = TakGame::new(game_settings.clone());

        let game_id = self.increment_game_id();
        let game_struct = Game {
            date,
            white,
            black,
            game,
            game_type,
            settings: game_settings,
            game_id,
            match_id,
        };
        self.games.insert(game_id, game_struct.clone());

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
    ) -> Result<DoActionSuccess, DoActionError> {
        let game_record = self
            .with_game(game_id, |game_entry| {
                let current_player = match player {
                    id if id == game_entry.white => TakPlayer::White,
                    id if id == game_entry.black => TakPlayer::Black,
                    _ => return Err(DoActionError::NotPlayersTurn),
                };

                if game_entry.game.current_player() != current_player {
                    return Err(DoActionError::NotPlayersTurn);
                }

                match game_entry.game.do_action(&action) {
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

    fn resign(&self, game_id: GameId, player: PlayerId) -> Result<Game, ResignError> {
        self.with_game(game_id, |game_entry| {
            let current_player = match player {
                id if id == game_entry.white => TakPlayer::White,
                id if id == game_entry.black => TakPlayer::Black,
                _ => return Err(ResignError::NotPlayersTurn),
            };

            if game_entry.game.current_player() != current_player {
                return Err(ResignError::NotPlayersTurn);
            }

            if let Err(_) = game_entry.game.resign(&current_player) {
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
    ) -> Result<OfferDrawSuccess, OfferDrawError> {
        let did_draw = self
            .with_game(game_id, |game_entry| {
                let current_player = match player {
                    id if id == game_entry.white => TakPlayer::White,
                    id if id == game_entry.black => TakPlayer::Black,
                    _ => return Err(OfferDrawError::NotAPlayerInGame),
                };

                let did_draw = match game_entry.game.offer_draw(&current_player, true) {
                    Ok(did_draw) => did_draw,
                    Err(_) => return Err(OfferDrawError::InvalidOffer),
                };

                Ok(did_draw)
            })
            .unwrap_or(Err(OfferDrawError::GameNotFound))?;
        if did_draw {
            let Some(ended_game) = self.handle_maybe_game_over(game_id) else {
                return Err(OfferDrawError::GameNotFound);
            };
            Ok(OfferDrawSuccess::GameDrawn(ended_game))
        } else {
            Ok(OfferDrawSuccess::DrawOffered)
        }
    }

    fn request_undo(&self, game_id: GameId, player: PlayerId) -> Result<bool, RequestUndoError> {
        let did_undo = self
            .with_game(game_id, |game_entry| {
                let current_player = match player {
                    id if id == game_entry.white => TakPlayer::White,
                    id if id == game_entry.black => TakPlayer::Black,
                    _ => return Err(RequestUndoError::NotAPlayerInGame),
                };

                match game_entry.game.request_undo(&current_player, true) {
                    Ok(did_undo) => Ok(did_undo),
                    Err(_) => return Err(RequestUndoError::InvalidRequest),
                }
            })
            .unwrap_or(Err(RequestUndoError::GameNotFound))?;

        self.handle_maybe_game_over(game_id);

        Ok(did_undo)
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
