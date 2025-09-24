use std::{
    ops::Add,
    sync::Arc,
    time::{Duration, Instant},
};

use dashmap::DashMap;
use tokio::select;
use tokio_util::sync::CancellationToken;

use crate::{
    ArcClientService, ArcGameRepository, ArcPlayerService, ServiceError, ServiceResult,
    client::ClientId,
    persistence::games::{GameEntity, GameUpdate},
    player::PlayerUsername,
    protocol::{ServerGameMessage, ServerMessage},
    seek::{GameType, Seek},
    tak::{TakAction, TakGame, TakPlayer, TakPos, TakVariant},
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

pub trait GameService {
    fn get_game_ids(&self) -> Vec<GameId>;
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
    fn observe_game(&self, id: &ClientId, game_id: &GameId) -> ServiceResult<()>;
    fn unobserve_game(&self, id: &ClientId, game_id: &GameId) -> ServiceResult<()>;
}

#[derive(Clone)]
pub struct GameServiceImpl {
    client_service: ArcClientService,
    player_service: ArcPlayerService,
    game_repository: ArcGameRepository,
    games: Arc<DashMap<GameId, Game>>,
    game_ended_tokens: Arc<DashMap<GameId, CancellationToken>>,
    game_spectators: Arc<DashMap<GameId, Vec<ClientId>>>,
    game_by_player: Arc<DashMap<PlayerUsername, GameId>>,
}

impl GameServiceImpl {
    pub fn new(
        client_service: ArcClientService,
        player_service: ArcPlayerService,
        game_repository: ArcGameRepository,
    ) -> Self {
        Self {
            client_service,
            player_service,
            game_repository,
            games: Arc::new(DashMap::new()),
            game_ended_tokens: Arc::new(DashMap::new()),
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
        &self,
        white: &PlayerUsername,
        black: &PlayerUsername,
        seek: &Seek,
    ) -> ServiceResult<GameId> {
        let white_player = self.player_service.fetch_player(white)?;
        let black_player = self.player_service.fetch_player(black)?;
        let game_entity = GameEntity {
            id: 0, // will be set by the database
            date: chrono::Utc::now().timestamp(),
            size: seek.game_settings.board_size as i32,
            player_white: white.clone(),
            player_black: black.clone(),
            notation: "".to_string(),
            result: "0-0".to_string(),
            timertime: seek.game_settings.time_control.contingent.as_secs() as i32,
            timerinc: seek.game_settings.time_control.increment.as_secs() as i32,
            rating_white: white_player.rating as i32,
            rating_black: black_player.rating as i32,
            unrated: seek.game_type == crate::seek::GameType::Unrated,
            tournament: seek.game_type == crate::seek::GameType::Tournament,
            komi: seek.game_settings.half_komi as i32,
            pieces: seek.game_settings.reserve_pieces as i32,
            capstones: seek.game_settings.reserve_capstones as i32,
            rating_change_white: -1000,
            rating_change_black: -1000,
            extra_time_amount: seek
                .game_settings
                .time_control
                .extra
                .as_ref()
                .map_or(0, |(_, extra_time)| extra_time.as_secs() as i32),
            extra_time_trigger: seek
                .game_settings
                .time_control
                .extra
                .as_ref()
                .map_or(0, |(trigger_move, _)| *trigger_move as i32),
        };
        let game_id = self
            .game_repository
            .create_game(&game_entity)
            .map_err(|e| ServiceError::Internal(format!("Failed to create game: {}", e)))?;
        Ok(game_id)
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

    fn save_to_database(&self, game: &Game) -> Result<(), String> {
        let notation = game
            .game
            .action_history
            .iter()
            .map(Self::move_to_database_string)
            .collect::<Vec<_>>()
            .join(",");
        let result = game.game.base.game_state.to_string();
        let update = GameUpdate {
            notation: Some(notation),
            result: Some(result),
        };
        self.game_repository
            .update_game(game.id, &update)
            .map_err(|e| format!("Failed to update game in database: {}", e))?;
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
        if let Some((_, token)) = self.game_ended_tokens.remove(game_id) {
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
            game_id: *game_id,
        };
        self.client_service
            .try_auth_protocol_broadcast(&game_remove_msg);

        if let Err(e) = self.save_to_database(&game) {
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
                let now = Instant::now();
                if !game_ref.game.is_ongoing() {
                    break;
                }

                let white_last_active = game_service
                    .client_service
                    .get_offline_since(&game_ref.white)
                    .unwrap_or(Some(now));
                let black_last_active = game_service
                    .client_service
                    .get_offline_since(&game_ref.black)
                    .unwrap_or(Some(now));

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
    fn observe_game(&self, id: &ClientId, game_id: &GameId) -> ServiceResult<()> {
        if self.games.get(game_id).is_none() {
            return ServiceError::not_found("Game ID not found");
        };

        let mut spectators = self
            .game_spectators
            .entry(*game_id)
            .or_insert_with(|| Vec::new());
        if !spectators.contains(&id) {
            spectators.push(id.clone());
        }
        drop(spectators);

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

        let game_new_msg = ServerMessage::GameList {
            add: true,
            game_id: id,
        };
        self.client_service
            .try_auth_protocol_broadcast(&game_new_msg);

        let game_start_msg = ServerMessage::GameStart { game_id: id };
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

    fn get_game_ids(&self) -> Vec<GameId> {
        self.games.iter().map(|entry| *entry.key()).collect()
    }

    fn get_game(&self, id: &GameId) -> Option<Game> {
        let game_ref = self.games.get(id);
        game_ref.as_ref().map(|g| g.value().clone())
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

#[derive(Clone, Default)]
pub struct MockGameService {}

impl GameService for MockGameService {
    fn get_game_ids(&self) -> Vec<GameId> {
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
    fn observe_game(&self, _id: &ClientId, _game_id: &GameId) -> ServiceResult<()> {
        Ok(())
    }
    fn unobserve_game(&self, _id: &ClientId, _game_id: &GameId) -> ServiceResult<()> {
        Ok(())
    }
}
