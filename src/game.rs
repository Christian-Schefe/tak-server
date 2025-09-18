use std::{
    sync::{Arc, LazyLock},
    time::{Duration, Instant},
};

use dashmap::DashMap;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use tokio::select;
use tokio_util::sync::CancellationToken;

use crate::{
    DatabaseError, ServiceError, ServiceResult,
    client::{ClientId, ClientService},
    player::PlayerUsername,
    protocol::{ServerGameMessage, ServerMessage},
    seek::{GameType, Seek},
    tak::{TakAction, TakGame, TakPlayer, TakPos, TakVariant},
};

static GAMES_DB_POOL: LazyLock<Pool<SqliteConnectionManager>> = LazyLock::new(|| {
    let db_path = std::env::var("TAK_GAMES_DB").expect("TAK_GAMES_DB env var not set");
    let manager = SqliteConnectionManager::file(db_path);
    Pool::builder()
        .max_size(5)
        .build(manager)
        .expect("Failed to create DB pool")
});

pub type GameId = u32;

#[derive(Clone, Debug)]
pub struct Game {
    pub id: GameId,
    pub white: PlayerUsername,
    pub black: PlayerUsername,
    pub game: TakGame,
    pub game_type: GameType,
}

pub trait GameService {
    fn get_games(&self) -> Vec<Game>;
    fn has_active_game(&self, player: &PlayerUsername) -> bool;
    fn get_active_game_of_player(&self, player: &PlayerUsername) -> Option<Game>;
    fn add_game_from_seek(&self, seek: &Seek, opponent: &PlayerUsername) -> ServiceResult<()>;
    fn try_do_action(
        &self,
        username: &PlayerUsername,
        game_id: &GameId,
        action: TakAction,
    ) -> ServiceResult<()>;
    fn resign_game(&self, username: &PlayerUsername, game_id: &GameId) -> ServiceResult<()>;
    fn offer_draw(
        &self,
        username: &PlayerUsername,
        game_id: &GameId,
        offer: bool,
    ) -> ServiceResult<()>;
    fn request_undo(
        &self,
        username: &PlayerUsername,
        game_id: &GameId,
        request: bool,
    ) -> ServiceResult<()>;
    fn observe_game(&self, id: &ClientId, game_id: &GameId) -> ServiceResult<()>;
    fn unobserve_game(&self, id: &ClientId, game_id: &GameId) -> ServiceResult<()>;
}

#[derive(Clone)]
pub struct GameServiceImpl {
    client_service: Arc<Box<dyn ClientService + Send + Sync>>,
    games: Arc<DashMap<GameId, Game>>,
    game_timeout_tokens: Arc<DashMap<GameId, CancellationToken>>,
    game_spectators: Arc<DashMap<GameId, Vec<ClientId>>>,
    game_by_player: Arc<DashMap<PlayerUsername, GameId>>,
}

impl GameServiceImpl {
    pub fn new(client_service: Arc<Box<dyn ClientService + Send + Sync>>) -> Self {
        Self {
            client_service,
            games: Arc::new(DashMap::new()),
            game_timeout_tokens: Arc::new(DashMap::new()),
            game_spectators: Arc::new(DashMap::new()),
            game_by_player: Arc::new(DashMap::new()),
        }
    }

    fn get_game_player(game: &Game, username: &PlayerUsername) -> ServiceResult<TakPlayer> {
        if &game.white == username {
            Ok(TakPlayer::White)
        } else if &game.black == username {
            Ok(TakPlayer::Black)
        } else {
            ServiceError::not_found("You are not a player in this game")
        }
    }

    fn get_opponent_username(game: &Game, player: &TakPlayer) -> PlayerUsername {
        match player {
            TakPlayer::White => game.black.clone(),
            TakPlayer::Black => game.white.clone(),
        }
    }

    fn insert_empty_game(
        white: &PlayerUsername,
        black: &PlayerUsername,
        seek: &Seek,
    ) -> ServiceResult<GameId> {
        let conn = GAMES_DB_POOL
            .get()
            .map_err(|e| DatabaseError::ConnectionError(e))?;
        let params = [
            chrono::Utc::now().naive_utc().to_string(),
            seek.game_settings.board_size.to_string(),
            white.to_string(),
            black.to_string(),
            seek.game_settings
                .time_control
                .contingent
                .as_secs()
                .to_string(),
            seek.game_settings
                .time_control
                .increment
                .as_secs()
                .to_string(),
            "".to_string(),
            "0-0".to_string(),
            "-1000".to_string(), //TODO: player ratings (see open question in readme)
            "-1000".to_string(),
            if seek.game_type == crate::seek::GameType::Unrated {
                "1"
            } else {
                "0"
            }
            .to_string(),
            if seek.game_type == crate::seek::GameType::Tournament {
                "1"
            } else {
                "0"
            }
            .to_string(),
            seek.game_settings.half_komi.to_string(),
            seek.game_settings.reserve_pieces.to_string(),
            seek.game_settings.reserve_capstones.to_string(),
            "-1000".to_string(),
            "-1000".to_string(),
            seek.game_settings
                .time_control
                .extra
                .as_ref()
                .map_or("0".to_string(), |(_, extra_time)| {
                    extra_time.as_secs().to_string()
                }),
            seek.game_settings
                .time_control
                .extra
                .as_ref()
                .map_or("0".to_string(), |(trigger_move, _)| {
                    trigger_move.to_string()
                }),
        ];
        conn.execute(
        "INSERT INTO games (date, size, player_white, player_black, timertime, timerinc, notation, result, rating_white, rating_black, unrated, tournament, komi, pieces, capstones, rating_change_white, rating_change_black, extra_time_amount, extra_time_trigger)  VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
        params
    )
    .map_err(|e| DatabaseError::QueryError(e))?;
        Ok(conn.last_insert_rowid() as GameId)
    }

    fn move_to_database_string(action: &TakAction) -> String {
        fn square_to_string(pos: &TakPos) -> String {
            format!(
                "{}{}",
                (b'A' + pos.x as u8) as char,
                (b'1' + pos.y as u8) as char,
            )
        }
        match action {
            TakAction::Place { pos, variant } => format!(
                "P {} {}",
                square_to_string(pos),
                match variant {
                    TakVariant::Flat => "",
                    TakVariant::Standing => "S",
                    TakVariant::Capstone => "C",
                },
            ),
            TakAction::Move { pos, dir, drops } => {
                let to_pos = pos.offset(dir, drops.len() as i32);
                let drops_str = drops
                    .iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<_>>()
                    .join("");
                format!(
                    "M {} {} {}",
                    square_to_string(pos),
                    square_to_string(&to_pos),
                    drops_str
                )
            }
        }
    }

    fn save_to_database(game: &Game) -> Result<(), String> {
        let conn = GAMES_DB_POOL
            .get()
            .map_err(|_| "Failed to get DB connection")?;
        let notation = game
            .game
            .action_history
            .iter()
            .map(Self::move_to_database_string)
            .collect::<Vec<_>>()
            .join(",");
        let result = game.game.base.game_state.to_string();
        let params = [notation, result, game.id.to_string()];
        conn.execute(
            "UPDATE games SET notation = ?1, result = ?2 WHERE id = ?3",
            params,
        )
        .map_err(|_| "Failed to update game in database")?;
        Ok(())
    }

    fn check_game_over(&self, game_id: &GameId) {
        let game_ref = self.games.get(game_id);
        let game = match game_ref.as_ref() {
            Some(g) if !g.game.is_ongoing() => g.value().clone(),
            _ => return,
        };
        drop(game_ref);

        println!(
            "Game {} is over: {}",
            game.id,
            game.game.base.game_state.to_string()
        );

        self.games.remove(game_id);
        if let Some((_, token)) = self.game_timeout_tokens.remove(game_id) {
            token.cancel();
        }
        self.game_by_player.remove(&game.white);
        self.game_by_player.remove(&game.black);

        let game_over_msg = ServerMessage::GameMessage {
            game_id: *game_id,
            message: ServerGameMessage::GameOver(game.game.base.game_state.clone()),
        };
        self.client_service
            .get_associated_client(&game.white)
            .map(|id| self.client_service.try_protocol_send(&id, &game_over_msg));
        self.client_service
            .get_associated_client(&game.black)
            .map(|id| self.client_service.try_protocol_send(&id, &game_over_msg));

        if let Some((_, spectators)) = self.game_spectators.remove(game_id) {
            self.client_service
                .try_protocol_multicast(&spectators, &game_over_msg);
        }

        let game_remove_msg = ServerMessage::GameList {
            add: false,
            game: game.clone(),
        };
        self.client_service
            .try_auth_protocol_broadcast(&game_remove_msg);

        if let Err(e) = Self::save_to_database(&game) {
            eprintln!("Failed to save game to database: {}", e);
        }
    }

    fn send_time_update(&self, game_id: &GameId) {
        let game_ref = self.games.get(game_id);
        let now = Instant::now();
        let (players, remaining) = match game_ref.as_ref() {
            Some(g) if g.game.is_ongoing() => (
                (g.white.clone(), g.black.clone()),
                g.game.get_time_remaining_both(now),
            ),
            _ => return,
        };
        drop(game_ref);

        let time_update_msg = ServerMessage::GameMessage {
            game_id: *game_id,
            message: ServerGameMessage::TimeUpdate { remaining },
        };
        self.client_service
            .get_associated_client(&players.0)
            .map(|id| self.client_service.try_protocol_send(&id, &time_update_msg));
        self.client_service
            .get_associated_client(&players.1)
            .map(|id| self.client_service.try_protocol_send(&id, &time_update_msg));
        if let Some(spectators) = self.game_spectators.get(game_id) {
            self.client_service
                .try_protocol_multicast(&spectators.value(), &time_update_msg);
        }
    }

    fn run_timeout_waiter(&self, game_id: GameId, cancel_token: CancellationToken) {
        let game_service = self.clone();
        tokio::spawn(async move {
            loop {
                let Some(mut game_ref) = game_service.games.get_mut(&game_id) else {
                    return;
                };
                let now = Instant::now();
                game_ref.game.check_timeout(now);
                if !game_ref.game.is_ongoing() {
                    break;
                }
                let min_duration_to_timeout = {
                    let (white_time, black_time) = game_ref.game.get_time_remaining_both(now);
                    white_time.min(black_time).max(Duration::from_millis(100))
                };
                drop(game_ref);
                select! {
                    _ = cancel_token.cancelled() => {
                        return;
                    }
                    _ = tokio::time::sleep(min_duration_to_timeout) => {}
                }
            }
            game_service.game_timeout_tokens.remove(&game_id);
            game_service.check_game_over(&game_id);
        });
    }
}

impl GameService for GameServiceImpl {
    fn observe_game(&self, id: &ClientId, game_id: &GameId) -> ServiceResult<()> {
        let Some(game_ref) = self.games.get(game_id) else {
            return ServiceError::not_found("Game ID not found");
        };
        let game = game_ref.value().clone();
        drop(game_ref);

        let mut spectators = self
            .game_spectators
            .entry(*game_id)
            .or_insert_with(|| Vec::new());
        if !spectators.contains(&id) {
            spectators.push(id.clone());
        }
        drop(spectators);

        let msg = ServerMessage::ObserveGame { game };
        self.client_service.try_protocol_send(id, &msg);

        Ok(())
    }

    fn unobserve_game(&self, id: &ClientId, game_id: &GameId) -> ServiceResult<()> {
        if let Some(mut spectators) = self.game_spectators.get_mut(game_id) {
            spectators.retain(|u| u != id);
        }
        Ok(())
    }

    fn offer_draw(
        &self,
        username: &PlayerUsername,
        game_id: &GameId,
        offer: bool,
    ) -> ServiceResult<()> {
        let Some(mut game_ref) = self.games.get_mut(game_id) else {
            return ServiceError::not_found("Game ID not found");
        };
        let player = Self::get_game_player(&game_ref, username)?;
        let did_draw = game_ref
            .game
            .offer_draw(&player, offer)
            .map_err(|e| ServiceError::NotPossible(e))?;
        let opponent = Self::get_opponent_username(&game_ref, &player);
        drop(game_ref);

        if !did_draw {
            let draw_offer_msg = ServerMessage::GameMessage {
                game_id: *game_id,
                message: ServerGameMessage::DrawOffer { offer },
            };
            self.client_service
                .get_associated_client(&opponent)
                .map(|id| self.client_service.try_protocol_send(&id, &draw_offer_msg));
        }

        self.check_game_over(game_id);
        Ok(())
    }

    fn request_undo(
        &self,
        username: &PlayerUsername,
        game_id: &GameId,
        request: bool,
    ) -> ServiceResult<()> {
        let Some(mut game_ref) = self.games.get_mut(game_id) else {
            return ServiceError::not_found("Game ID not found");
        };
        let player = Self::get_game_player(&game_ref, username)?;
        let did_undo = game_ref
            .game
            .request_undo(&player, request)
            .map_err(|e| ServiceError::NotPossible(e))?;
        let opponent = Self::get_opponent_username(&game_ref, &player);
        drop(game_ref);

        if !did_undo {
            let undo_request_msg = ServerMessage::GameMessage {
                game_id: *game_id,
                message: ServerGameMessage::UndoRequest { request },
            };
            self.client_service
                .get_associated_client(&opponent)
                .map(|id| {
                    self.client_service
                        .try_protocol_send(&id, &undo_request_msg)
                });
        } else {
            self.send_time_update(game_id);

            let undo_msg = ServerMessage::GameMessage {
                game_id: *game_id,
                message: ServerGameMessage::Undo,
            };
            self.client_service
                .get_associated_client(&username)
                .map(|id| self.client_service.try_protocol_send(&id, &undo_msg));
            self.client_service
                .get_associated_client(&opponent)
                .map(|id| self.client_service.try_protocol_send(&id, &undo_msg));

            if let Some(spectators) = self.game_spectators.get(game_id) {
                self.client_service
                    .try_protocol_multicast(&spectators, &undo_msg);
            }
        }

        Ok(())
    }

    fn add_game_from_seek(&self, seek: &Seek, opponent: &PlayerUsername) -> ServiceResult<()> {
        if &seek.creator == opponent {
            return ServiceError::not_possible("You cannot accept your own seek");
        }
        if self.has_active_game(&seek.creator) {
            return ServiceError::not_possible("Player is already in a game");
        }
        if self.has_active_game(opponent) {
            return ServiceError::not_possible("Player is already in a game");
        }
        let (white, black) = match &seek.color {
            Some(TakPlayer::White) => (seek.creator.clone(), opponent.clone()),
            Some(TakPlayer::Black) => (opponent.clone(), seek.creator.clone()),
            None => {
                if rand::random() {
                    (seek.creator.clone(), opponent.clone())
                } else {
                    (opponent.clone(), seek.creator.clone())
                }
            }
        };
        let id = Self::insert_empty_game(&white, &black, seek)?;
        let game = Game {
            id,
            white,
            black,
            game: TakGame::new(seek.game_settings.clone()),
            game_type: seek.game_type.clone(),
        };
        self.games.insert(id, game.clone());
        self.game_by_player.insert(seek.creator.clone(), id);
        self.game_by_player.insert(opponent.clone(), id);

        println!("Game {} created", id);

        let cancel_token = CancellationToken::new();
        self.game_timeout_tokens.insert(id, cancel_token.clone());
        self.run_timeout_waiter(id, cancel_token);

        let game_new_msg = ServerMessage::GameList {
            add: true,
            game: game.clone(),
        };
        self.client_service
            .try_auth_protocol_broadcast(&game_new_msg);

        let game_start_msg = ServerMessage::GameStart { game };
        self.client_service
            .get_associated_client(&seek.creator)
            .map(|id| self.client_service.try_protocol_send(&id, &game_start_msg));
        self.client_service
            .get_associated_client(&opponent)
            .map(|id| self.client_service.try_protocol_send(&id, &game_start_msg));
        Ok(())
    }

    fn try_do_action(
        &self,
        username: &PlayerUsername,
        game_id: &GameId,
        action: TakAction,
    ) -> ServiceResult<()> {
        let Some(mut game_ref) = self.games.get_mut(game_id) else {
            return ServiceError::not_found("Game ID not found");
        };
        let player = Self::get_game_player(&game_ref, username)?;
        if game_ref.game.base.current_player != player {
            return ServiceError::not_possible("It's not your turn");
        }
        game_ref
            .game
            .do_action(&action)
            .map_err(|e| ServiceError::NotPossible(e))?;
        let opponent = Self::get_opponent_username(&game_ref, &player);
        drop(game_ref);

        self.send_time_update(game_id);

        let action_msg = ServerMessage::GameMessage {
            game_id: *game_id,
            message: ServerGameMessage::Action(action.clone()),
        };

        self.client_service
            .get_associated_client(&opponent)
            .map(|id| self.client_service.try_protocol_send(&id, &action_msg));
        if let Some(spectators) = self.game_spectators.get(game_id) {
            self.client_service
                .try_protocol_multicast(&spectators.value(), &action_msg);
        }

        self.check_game_over(game_id);
        Ok(())
    }

    fn get_games(&self) -> Vec<Game> {
        self.games
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    fn has_active_game(&self, player: &PlayerUsername) -> bool {
        self.game_by_player.contains_key(player)
    }

    fn get_active_game_of_player(&self, player: &PlayerUsername) -> Option<Game> {
        for entry in self.games.iter() {
            let game = entry.value();
            if &game.white == player || &game.black == player {
                return Some(game.clone());
            }
        }
        None
    }

    fn resign_game(&self, username: &PlayerUsername, game_id: &GameId) -> ServiceResult<()> {
        let Some(mut game_ref) = self.games.get_mut(game_id) else {
            return ServiceError::not_found("Game ID not found");
        };
        let player = Self::get_game_player(&game_ref, username)?;
        game_ref
            .game
            .resign(&player)
            .map_err(|e| ServiceError::NotPossible(e))?;
        drop(game_ref);

        self.check_game_over(game_id);
        Ok(())
    }
}
