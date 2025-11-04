use std::{
    ops::Add,
    sync::Arc,
    time::{Duration, Instant},
};

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use tak_core::{
    TakAction, TakGame, TakGameSettings, TakGameState, TakPlayer, ptn::game_state_to_string,
};
use tokio::select;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    ServiceError, ServiceResult,
    player::{ArcPlayerService, PlayerUsername},
    seek::Seek,
    transport::{
        ArcPlayerConnectionService, ArcTransportService, ServerGameMessage, ServerMessage,
    },
    util::ManyManyDashMap,
};

pub type GameId = i64;

const GAME_DISCONNECT_TIMEOUT: Duration = Duration::from_secs(120);
const GAME_TOURNAMENT_DISCONNECT_TIMEOUT: Duration = Duration::from_secs(900);

#[derive(Clone, Debug)]
pub struct Game {
    pub id: GameId,
    pub white: PlayerUsername,
    pub black: PlayerUsername,
    pub game: TakGame,
    pub game_type: GameType,
}

pub struct GameRecord {
    pub date: DateTime<Utc>,
    pub white: PlayerUsername,
    pub black: PlayerUsername,
    pub white_rating: f64,
    pub black_rating: f64,
    pub settings: TakGameSettings,
    pub game_type: GameType,
    pub result: TakGameState,
    pub moves: Vec<TakAction>,
}

pub struct GameRecordUpdate {
    pub result: TakGameState,
    pub moves: Vec<TakAction>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GameType {
    Unrated,
    Rated,
    Tournament,
}

pub type SpectatorId = Uuid;

pub type ArcGameRepository = Arc<Box<dyn GameRepository + Send + Sync + 'static>>;
pub trait GameRepository {
    fn create_game(&self, game: &GameRecord) -> ServiceResult<GameId>;
    fn update_game(&self, id: GameId, update: &GameRecordUpdate) -> ServiceResult<()>;
}

pub type ArcGameService = Arc<Box<dyn GameService + Send + Sync + 'static>>;
pub trait GameService {
    fn get_game_ids(&self) -> Vec<GameId>;
    fn get_games(&self) -> Vec<Game>;
    fn get_game(&self, id: &GameId) -> Option<Game>;
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
    fn observe_game(&self, id: &SpectatorId, game_id: &GameId) -> ServiceResult<()>;
    fn unobserve_game(&self, id: &SpectatorId, game_id: &GameId) -> ServiceResult<()>;
    fn unobserve_all(&self, id: &SpectatorId) -> ServiceResult<()>;
}

#[derive(Clone)]
pub struct GameServiceImpl {
    transport_service: ArcTransportService,
    player_connection_service: ArcPlayerConnectionService,
    player_service: ArcPlayerService,
    game_repository: ArcGameRepository,
    games: Arc<DashMap<GameId, Game>>,
    game_ended_tokens: Arc<DashMap<GameId, CancellationToken>>,
    game_spectators: Arc<ManyManyDashMap<GameId, SpectatorId>>,
    game_by_player: Arc<DashMap<PlayerUsername, GameId>>,
}

impl GameServiceImpl {
    pub fn new(
        transport_service: ArcTransportService,
        player_connection_service: ArcPlayerConnectionService,
        player_service: ArcPlayerService,
        game_repository: ArcGameRepository,
    ) -> Self {
        Self {
            transport_service,
            player_connection_service,
            player_service,
            game_repository,
            games: Arc::new(DashMap::new()),
            game_ended_tokens: Arc::new(DashMap::new()),
            game_spectators: Arc::new(ManyManyDashMap::new()),
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
        &self,
        white: &PlayerUsername,
        black: &PlayerUsername,
        seek: &Seek,
    ) -> ServiceResult<GameId> {
        let white_player = self.player_service.fetch_player_data(white)?;
        let black_player = self.player_service.fetch_player_data(black)?;
        let game_record = GameRecord {
            date: chrono::Utc::now(),
            white: white.clone(),
            black: black.clone(),
            white_rating: white_player.rating,
            black_rating: black_player.rating,
            settings: seek.game_settings.clone(),
            game_type: seek.game_type.clone(),
            result: TakGameState::Ongoing,
            moves: vec![],
        };
        let game_id = self.game_repository.create_game(&game_record)?;
        Ok(game_id)
    }

    fn save_to_database(&self, game: &Game) -> ServiceResult<()> {
        let update = GameRecordUpdate {
            moves: game.game.action_history.clone(),
            result: game.game.base.game_state.clone(),
        };
        self.game_repository.update_game(game.id, &update)?;
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
            game_state_to_string(&game.game.base.game_state)
        );

        self.send_time_update(game_id);

        self.games.remove(game_id);
        if let Some((_, token)) = self.game_ended_tokens.remove(game_id) {
            token.cancel();
        }
        self.game_by_player.remove(&game.white);
        self.game_by_player.remove(&game.black);

        let game_over_msg = ServerMessage::GameMessage {
            game_id: *game_id,
            message: ServerGameMessage::GameOver {
                game_state: game.game.base.game_state.clone(),
            },
        };
        self.transport_service
            .try_player_send(&game.white, &game_over_msg);
        self.transport_service
            .try_player_send(&game.black, &game_over_msg);

        let spectators = self.game_spectators.remove_key(game_id);
        self.transport_service
            .try_spectator_multicast(&spectators, &game_over_msg);

        let game_remove_msg = ServerMessage::GameList {
            add: false,
            game: game.clone(),
        };
        self.transport_service
            .try_player_broadcast(&game_remove_msg);

        if let Err(e) = self.save_to_database(&game) {
            eprintln!("Failed to save game to database: {}", e);
        }
    }

    fn send_time_update(&self, game_id: &GameId) {
        let game_ref = self.games.get(game_id);
        let now = Instant::now();
        let (players, remaining) = match game_ref.as_ref() {
            Some(g) => (
                (g.white.clone(), g.black.clone()),
                g.game.get_time_remaining_both(now),
            ),
            None => return,
        };
        drop(game_ref);

        let time_update_msg = ServerMessage::GameMessage {
            game_id: *game_id,
            message: ServerGameMessage::TimeUpdate {
                remaining_white: remaining.0,
                remaining_black: remaining.1,
            },
        };
        self.transport_service
            .try_player_send(&players.0, &time_update_msg);
        self.transport_service
            .try_player_send(&players.1, &time_update_msg);

        let spectators = self.game_spectators.get_by_key(game_id);
        self.transport_service
            .try_spectator_multicast(&spectators, &time_update_msg);
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
                    white_time.min(black_time).add(Duration::from_millis(100))
                };
                drop(game_ref);
                println!(
                    "Game {}: waiting for timeout check in {:?}",
                    game_id, min_duration_to_timeout
                );
                select! {
                    _ = cancel_token.cancelled() => {
                        return;
                    }
                    _ = tokio::time::sleep(min_duration_to_timeout) => {}
                }
            }
            game_service.check_game_over(&game_id);
        });
    }

    fn run_disconnect_waiter(&self, game_id: GameId, cancel_token: CancellationToken) {
        let game_service = self.clone();
        tokio::spawn(async move {
            loop {
                let Some(mut game_ref) = game_service.games.get_mut(&game_id) else {
                    return;
                };
                if !game_ref.game.is_ongoing() {
                    break;
                }

                let white_last_active = game_service
                    .player_connection_service
                    .get_last_connected(&game_ref.white);
                let black_last_active = game_service
                    .player_connection_service
                    .get_last_connected(&game_ref.black);

                let timeout = if game_ref.game_type == GameType::Tournament {
                    GAME_TOURNAMENT_DISCONNECT_TIMEOUT
                } else {
                    GAME_DISCONNECT_TIMEOUT
                };

                let white_min_timeout =
                    white_last_active.map(|t| timeout.saturating_sub(t.elapsed()));
                let black_min_timeout =
                    black_last_active.map(|t| timeout.saturating_sub(t.elapsed()));

                if white_min_timeout.is_none_or(|t| t.is_zero()) {
                    game_ref.game.resign(&TakPlayer::White).ok();
                    break;
                } else if black_min_timeout.is_none_or(|t| t.is_zero()) {
                    game_ref.game.resign(&TakPlayer::Black).ok();
                    break;
                }
                drop(game_ref);

                let min_duration_to_timeout = white_min_timeout
                    .unwrap()
                    .min(black_min_timeout.unwrap())
                    .add(Duration::from_millis(100));

                println!(
                    "Game {}: waiting for disconnect check in {:?}",
                    game_id, min_duration_to_timeout
                );
                select! {
                    _ = cancel_token.cancelled() => {
                        return;
                    }
                    _ = tokio::time::sleep(min_duration_to_timeout) => {}
                }
            }
            game_service.check_game_over(&game_id);
        });
    }
}

impl GameService for GameServiceImpl {
    fn observe_game(&self, id: &SpectatorId, game_id: &GameId) -> ServiceResult<()> {
        if self.games.get(game_id).is_none() {
            return ServiceError::not_found("Game ID not found");
        };

        self.game_spectators.insert(*game_id, *id);
        Ok(())
    }

    fn unobserve_game(&self, id: &SpectatorId, game_id: &GameId) -> ServiceResult<()> {
        self.game_spectators.remove(game_id, id);
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
            self.transport_service
                .try_player_send(&opponent, &draw_offer_msg);
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
            self.transport_service
                .try_player_send(&opponent, &undo_request_msg);
        } else {
            self.send_time_update(game_id);

            let undo_msg = ServerMessage::GameMessage {
                game_id: *game_id,
                message: ServerGameMessage::Undo,
            };
            self.transport_service.try_player_send(&username, &undo_msg);
            self.transport_service.try_player_send(&opponent, &undo_msg);

            let spectators = self.game_spectators.get_by_key(game_id);
            self.transport_service
                .try_spectator_multicast(&spectators, &undo_msg);
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
        let id = self.insert_empty_game(&white, &black, seek)?;
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
        self.game_ended_tokens.insert(id, cancel_token.clone());
        self.run_disconnect_waiter(id, cancel_token.clone());
        self.run_timeout_waiter(id, cancel_token);

        let game_new_msg = ServerMessage::GameList { add: true, game };
        self.transport_service.try_player_broadcast(&game_new_msg);

        let game_start_msg = ServerMessage::GameStart { game_id: id };

        self.transport_service
            .try_player_send(&seek.creator, &game_start_msg);

        self.transport_service
            .try_player_send(&opponent, &game_start_msg);

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
            message: ServerGameMessage::Action {
                action: action.clone(),
            },
        };

        self.transport_service
            .try_player_send(&opponent, &action_msg);
        let spectators = self.game_spectators.get_by_key(game_id);
        self.transport_service
            .try_spectator_multicast(&spectators, &action_msg);

        self.check_game_over(game_id);
        Ok(())
    }

    fn get_game_ids(&self) -> Vec<GameId> {
        self.games.iter().map(|entry| *entry.key()).collect()
    }

    fn get_game(&self, id: &GameId) -> Option<Game> {
        let game_ref = self.games.get(id);
        game_ref.as_ref().map(|g| g.value().clone())
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

    fn unobserve_all(&self, id: &SpectatorId) -> ServiceResult<()> {
        self.game_spectators.remove_value(id);
        Ok(())
    }
}

#[derive(Clone, Default)]
pub struct MockGameService {}

impl GameService for MockGameService {
    fn get_game_ids(&self) -> Vec<GameId> {
        vec![]
    }
    fn get_games(&self) -> Vec<Game> {
        vec![]
    }
    fn get_game(&self, _id: &GameId) -> Option<Game> {
        None
    }
    fn has_active_game(&self, _player: &PlayerUsername) -> bool {
        false
    }
    fn get_active_game_of_player(&self, _player: &PlayerUsername) -> Option<Game> {
        None
    }
    fn add_game_from_seek(&self, _seek: &Seek, _opponent: &PlayerUsername) -> ServiceResult<()> {
        Ok(())
    }
    fn try_do_action(
        &self,
        _username: &PlayerUsername,
        _game_id: &GameId,
        _action: TakAction,
    ) -> ServiceResult<()> {
        Ok(())
    }
    fn resign_game(&self, _username: &PlayerUsername, _game_id: &GameId) -> ServiceResult<()> {
        Ok(())
    }
    fn offer_draw(
        &self,
        _username: &PlayerUsername,
        _game_id: &GameId,
        _offer: bool,
    ) -> ServiceResult<()> {
        Ok(())
    }
    fn request_undo(
        &self,
        _username: &PlayerUsername,
        _game_id: &GameId,
        _request: bool,
    ) -> ServiceResult<()> {
        Ok(())
    }
    fn observe_game(&self, _id: &SpectatorId, _game_id: &GameId) -> ServiceResult<()> {
        Ok(())
    }
    fn unobserve_game(&self, _id: &SpectatorId, _game_id: &GameId) -> ServiceResult<()> {
        Ok(())
    }
    fn unobserve_all(&self, _id: &SpectatorId) -> ServiceResult<()> {
        Ok(())
    }
}
