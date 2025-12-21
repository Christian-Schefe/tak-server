use std::sync::{Arc, Mutex};

use dashmap::DashMap;
use tak_core::{TakAction, TakActionRecord, TakGame, TakGameSettings, TakGameState, TakPlayer};

use crate::domain::{GameId, GameType, PlayerId};

#[derive(Clone, Debug)]
pub struct Game {
    pub white: PlayerId,
    pub black: PlayerId,
    pub game: TakGame,
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

#[derive(Clone, Debug)]
pub struct FinishedGame {
    pub white: PlayerId,
    pub black: PlayerId,
    pub settings: TakGameSettings,
    pub game_type: GameType,
    pub result: TakGameState,
    pub moves: Vec<TakActionRecord>,
}

pub trait GameService {
    fn create_game(
        &self,
        player1: PlayerId,
        player2: PlayerId,
        color: Option<TakPlayer>,
        game_type: GameType,
        game_settings: TakGameSettings,
    ) -> Result<(GameId, Game), CreateGameError>;
    fn get_game_by_id(&self, game_id: GameId) -> Option<Game>;
    fn get_games(&self) -> Vec<(GameId, Game)>;
    fn do_action(
        &self,
        game_id: GameId,
        player: PlayerId,
        action: TakAction,
    ) -> Result<TakActionRecord, DoActionError>;
    fn resign(&self, game_id: GameId, player: PlayerId) -> Result<(), ResignError>;
    fn request_undo(&self, game_id: GameId, player: PlayerId) -> Result<bool, RequestUndoError>;
    fn offer_draw(&self, game_id: GameId, player: PlayerId) -> Result<bool, OfferDrawError>;
    fn take_events(&self) -> Vec<GameEvent>;
}

pub enum GameEvent {
    Started(GameId, Game),
    MovePlayed(GameId, TakActionRecord),
    MoveUndone(GameId),
    Ended(GameId, FinishedGame),
}

pub enum CreateGameError {
    InvalidSettings,
    InvalidPlayers,
}

pub enum DoActionError {
    GameNotFound,
    NotPlayersTurn,
    InvalidAction,
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

pub enum RequestUndoError {
    GameNotFound,
    NotAPlayerInGame,
    InvalidRequest,
}

pub struct GameServiceImpl {
    games: Arc<DashMap<GameId, Game>>,
    events: Arc<Mutex<Vec<GameEvent>>>,
    next_game_id: Arc<Mutex<GameId>>,
}

impl GameServiceImpl {
    pub fn new() -> Self {
        Self {
            games: Arc::new(DashMap::new()),
            events: Arc::new(Mutex::new(Vec::new())),
            next_game_id: Arc::new(Mutex::new(GameId(0))),
        }
    }

    fn add_event(&self, event: GameEvent) {
        let mut events = self.events.lock().unwrap();
        events.push(event);
    }

    fn increment_game_id(&self) -> GameId {
        let mut id_lock = self.next_game_id.lock().unwrap();
        let game_id = *id_lock;
        id_lock.0 += 1;
        game_id
    }

    fn handle_maybe_game_over(&self, game_id: GameId) {
        if let Some(ended_game) = self
            .with_game(game_id, |game_entry| {
                if !game_entry.game.is_ongoing() {
                    Some(FinishedGame {
                        white: game_entry.white,
                        black: game_entry.black,
                        settings: game_entry.game.base.settings.clone(),
                        game_type: game_entry.game_type.clone(),
                        result: game_entry.game.base.game_state.clone(),
                        moves: game_entry.game.action_history.clone(),
                    })
                } else {
                    None
                }
            })
            .flatten()
        {
            self.games.remove(&game_id);
            self.add_event(GameEvent::Ended(game_id, ended_game));
        }
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
        player1: PlayerId,
        player2: PlayerId,
        color: Option<TakPlayer>,
        game_type: GameType,
        game_settings: TakGameSettings,
    ) -> Result<(GameId, Game), CreateGameError> {
        if !game_settings.is_valid() {
            return Err(CreateGameError::InvalidSettings);
        }
        if player1 == player2 {
            return Err(CreateGameError::InvalidPlayers);
        }

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

        let game_struct = Game {
            white,
            black,
            game,
            game_type,
        };
        let game_id = self.increment_game_id();
        self.games.insert(game_id, game_struct.clone());

        self.add_event(GameEvent::Started(game_id, game_struct.clone()));
        Ok((game_id, game_struct))
    }

    fn get_game_by_id(&self, game_id: GameId) -> Option<Game> {
        self.games.get(&game_id).map(|entry| entry.clone())
    }

    fn get_games(&self) -> Vec<(GameId, Game)> {
        self.games
            .iter()
            .map(|entry| (*entry.key(), entry.clone()))
            .collect()
    }

    fn do_action(
        &self,
        game_id: GameId,
        player: PlayerId,
        action: TakAction,
    ) -> Result<TakActionRecord, DoActionError> {
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

                let game = &mut game_entry.game;
                let game_record = match game.do_action(&action) {
                    Ok(record) => record,
                    Err(_) => return Err(DoActionError::InvalidAction),
                };

                self.add_event(GameEvent::MovePlayed(game_id, game_record.clone()));
                Ok(game_record.clone())
            })
            .unwrap_or(Err(DoActionError::GameNotFound))?;

        self.handle_maybe_game_over(game_id);

        Ok(game_record)
    }

    fn resign(&self, game_id: GameId, player: PlayerId) -> Result<(), ResignError> {
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

        self.handle_maybe_game_over(game_id);

        Ok(())
    }

    fn offer_draw(&self, game_id: GameId, player: PlayerId) -> Result<bool, OfferDrawError> {
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
        self.handle_maybe_game_over(game_id);

        Ok(did_draw)
    }

    fn request_undo(&self, game_id: GameId, player: PlayerId) -> Result<bool, RequestUndoError> {
        let did_undo = self
            .with_game(game_id, |game_entry| {
                let current_player = match player {
                    id if id == game_entry.white => TakPlayer::White,
                    id if id == game_entry.black => TakPlayer::Black,
                    _ => return Err(RequestUndoError::NotAPlayerInGame),
                };

                let did_undo = match game_entry.game.request_undo(&current_player, true) {
                    Ok(did_undo) => did_undo,
                    Err(_) => return Err(RequestUndoError::InvalidRequest),
                };

                if did_undo {
                    self.add_event(GameEvent::MoveUndone(game_id));
                }

                Ok(did_undo)
            })
            .unwrap_or(Err(RequestUndoError::GameNotFound))?;

        self.handle_maybe_game_over(game_id);

        Ok(did_undo)
    }

    fn take_events(&self) -> Vec<GameEvent> {
        let mut events = self.events.lock().unwrap();
        std::mem::take(&mut *events)
    }
}
