use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use tak_core::{TakAction, TakActionRecord, TakGame, TakGameSettings, TakGameState, TakPlayer};

use crate::{
    domain::{GameId, GameType, ListenerId, PlayerId},
    util::ManyManyDashMap,
};

#[derive(Clone, Debug)]
pub struct Game {
    pub id: GameId,
    pub white: PlayerId,
    pub black: PlayerId,
    pub game: TakGame,
    pub game_type: GameType,
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

pub struct GameRecord {
    pub date: DateTime<Utc>,
    pub white: PlayerId,
    pub black: PlayerId,
    pub rating_info: Option<GameRatingInfo>,
    pub settings: TakGameSettings,
    pub game_type: GameType,
    pub result: TakGameState,
    pub moves: Vec<TakActionRecord>,
}

pub struct GameRatingInfo {
    pub white_rating: i32,
    pub black_rating: i32,
    pub rating_change_white: i32,
    pub rating_change_black: i32,
}

pub trait GameService {
    fn create_game(
        &self,
        player1: PlayerId,
        player2: PlayerId,
        color: Option<TakPlayer>,
        game_type: GameType,
        game_settings: TakGameSettings,
    ) -> Result<GameId, CreateGameError>;
    fn get_game_of_player(&self, player: PlayerId) -> Option<GameId>;
    fn observe_game(&self, game_id: GameId, listener_id: ListenerId) -> bool;
    fn unobserve_game(&self, game_id: GameId, listener_id: ListenerId) -> bool;
    fn unobserve_all_games(&self, listener_id: ListenerId);
    fn do_action(
        &self,
        game_id: GameId,
        player: PlayerId,
        action: TakAction,
    ) -> Result<(), DoActionError>;
    fn finalize_game(&self, game_id: GameId, rating_info: GameRatingInfo);
    fn take_events(&self) -> Vec<GameEvent>;
}

pub trait GameRepository {
    fn get_next_game_id(&self, game: GameRecord) -> GameId;
    fn save_finished_game(&self, game_id: GameId, game: FinishedGame, rating_info: GameRatingInfo);
}

pub enum GameEvent {
    Started(Game),
    MovePlayed(GameId, TakActionRecord),
    Ended(Game),
}

pub enum CreateGameError {
    InvalidSettings,
    InvalidPlayers,
    PlayerInGame,
}

pub enum DoActionError {
    GameNotFound,
    NotPlayersTurn,
    InvalidAction,
}

pub struct GameServiceImpl<G: GameRepository> {
    repository: Arc<G>,
    games: Arc<DashMap<GameId, Game>>,
    game_spectators: Arc<ManyManyDashMap<GameId, ListenerId>>,
    game_by_player: Arc<DashMap<PlayerId, GameId>>,
    ended_games: Arc<DashMap<GameId, FinishedGame>>,
    events: Arc<Mutex<Vec<GameEvent>>>,
}

impl<G: GameRepository> GameServiceImpl<G> {
    pub fn new(repository: Arc<G>) -> Self {
        Self {
            repository,
            games: Arc::new(DashMap::new()),
            game_spectators: Arc::new(ManyManyDashMap::new()),
            game_by_player: Arc::new(DashMap::new()),
            ended_games: Arc::new(DashMap::new()),
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn add_event(&self, event: GameEvent) {
        let mut events = self.events.lock().unwrap();
        events.push(event);
    }
}

impl<G: GameRepository> GameService for GameServiceImpl<G> {
    fn create_game(
        &self,
        player1: PlayerId,
        player2: PlayerId,
        color: Option<TakPlayer>,
        game_type: GameType,
        game_settings: TakGameSettings,
    ) -> Result<GameId, CreateGameError> {
        if !game_settings.is_valid() {
            return Err(CreateGameError::InvalidSettings);
        }
        if player1 == player2 {
            return Err(CreateGameError::InvalidPlayers);
        }
        if self.game_by_player.contains_key(&player1) || self.game_by_player.contains_key(&player2)
        {
            return Err(CreateGameError::PlayerInGame);
        }
        let game = TakGame::new(game_settings.clone());
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
        let game_record = GameRecord {
            date: Utc::now(),
            white,
            black,
            rating_info: None,
            settings: game_settings,
            game_type: game_type.clone(),
            result: TakGameState::Ongoing,
            moves: vec![],
        };

        let game_id = self.repository.get_next_game_id(game_record);
        let game_struct = Game {
            id: game_id,
            white,
            black,
            game,
            game_type,
        };
        self.games.insert(game_id, game_struct.clone());
        self.game_by_player.insert(white, game_id);
        self.game_by_player.insert(black, game_id);

        self.add_event(GameEvent::Started(game_struct));

        Ok(game_id)
    }

    fn get_game_of_player(&self, player: PlayerId) -> Option<GameId> {
        Some(self.game_by_player.get(&player)?.clone())
    }

    fn observe_game(&self, game_id: GameId, listener_id: ListenerId) -> bool {
        if !self.games.contains_key(&game_id) {
            return false;
        }
        self.game_spectators.insert(game_id, listener_id);
        true
    }

    fn unobserve_game(&self, game_id: GameId, listener_id: ListenerId) -> bool {
        self.game_spectators.remove(&game_id, &listener_id)
    }

    fn unobserve_all_games(&self, listener_id: ListenerId) {
        self.game_spectators.remove_value(&listener_id);
    }

    fn do_action(
        &self,
        game_id: GameId,
        player: PlayerId,
        action: TakAction,
    ) -> Result<(), DoActionError> {
        let mut game_entry = self
            .games
            .get_mut(&game_id)
            .ok_or(DoActionError::GameNotFound)?;

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

        self.add_event(GameEvent::MovePlayed(game_id, game_record));

        if !game_entry.game.is_ongoing() {
            self.add_event(GameEvent::Ended(game_entry.clone()));
            self.game_by_player.remove(&game_entry.white);
            self.game_by_player.remove(&game_entry.black);
            let ended_game = FinishedGame {
                white: game_entry.white,
                black: game_entry.black,
                settings: game_entry.game.base.settings.clone(),
                game_type: game_entry.game_type.clone(),
                result: game_entry.game.base.game_state.clone(),
                moves: game_entry.game.action_history.clone(),
            };
            drop(game_entry);
            self.games.remove(&game_id);
            self.ended_games.insert(game_id, ended_game);
        }

        Ok(())
    }

    fn finalize_game(&self, game_id: GameId, rating_info: GameRatingInfo) {
        if let Some((_, ended_game)) = self.ended_games.remove(&game_id) {
            self.repository
                .save_finished_game(game_id, ended_game, rating_info);
        }
    }

    fn take_events(&self) -> Vec<GameEvent> {
        let mut events = self.events.lock().unwrap();
        std::mem::take(&mut *events)
    }
}
