use std::{
    ops::Add,
    sync::Arc,
    time::{Duration, Instant},
};

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use log::{debug, error, info};
use tak_core::{
    TakAction, TakActionRecord, TakGame, TakGameSettings, TakGameState, TakPlayer,
    ptn::game_state_to_string,
};
use tokio::select;
use tokio_util::sync::CancellationToken;

use crate::{
    ServiceError, ServiceResult,
    player::{ArcPlayerService, Player, PlayerUsername},
    rating::GameRatingInfo,
    seek::Seek,
    transport::{
        ArcPlayerConnectionService, ArcTransportService, ListenerId, ServerGameMessage,
        ServerMessage, do_player_broadcast, do_player_send,
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
    pub rating_info: Option<GameRatingInfo>,
    pub settings: TakGameSettings,
    pub game_type: GameType,
    pub result: TakGameState,
    pub moves: Vec<TakActionRecord>,
}

pub struct GameResultUpdate {
    pub result: TakGameState,
    pub moves: Vec<TakActionRecord>,
}

pub struct GameRatingUpdate {
    pub rating_info: GameRatingInfo,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GameType {
    Unrated,
    Rated,
    Tournament,
}

pub type ArcGameRepository = Arc<Box<dyn GameRepository + Send + Sync + 'static>>;

#[async_trait::async_trait]
pub trait GameRepository {
    async fn create_game(
        &self,
        game: &GameRecord,
        player_white: &Player,
        player_black: &Player,
    ) -> ServiceResult<GameId>;
    async fn update_game_result(&self, id: GameId, update: &GameResultUpdate) -> ServiceResult<()>;
    async fn update_game_rating(
        &self,
        id: GameId,
        rating_update: &GameRatingUpdate,
    ) -> ServiceResult<()>;
    async fn get_games(&self) -> ServiceResult<Vec<(GameId, GameRecord)>>;
}

pub type ArcGameService = Arc<Box<dyn GameService + Send + Sync + 'static>>;

#[async_trait::async_trait]
pub trait GameService {
    fn get_game_ids(&self) -> Vec<GameId>;
    fn get_games(&self) -> Vec<Game>;
    fn get_game(&self, id: &GameId) -> Option<Game>;
    fn has_active_game(&self, player: &PlayerUsername) -> bool;
    fn get_active_game_of_player(&self, player: &PlayerUsername) -> Option<Game>;
    async fn add_game_from_seek(&self, seek: &Seek, opponent: &PlayerUsername)
    -> ServiceResult<()>;
    async fn try_do_action(
        &self,
        player: &PlayerUsername,
        game_id: &GameId,
        action: TakAction,
    ) -> ServiceResult<()>;
    async fn resign_game(&self, player: &PlayerUsername, game_id: &GameId) -> ServiceResult<()>;
    async fn offer_draw(
        &self,
        player: &PlayerUsername,
        game_id: &GameId,
        offer: bool,
    ) -> ServiceResult<()>;
    async fn request_undo(
        &self,
        player: &PlayerUsername,
        game_id: &GameId,
        request: bool,
    ) -> ServiceResult<()>;
    fn observe_game(&self, id: ListenerId, game_id: &GameId) -> ServiceResult<()>;
    fn unobserve_game(&self, id: ListenerId, game_id: &GameId) -> ServiceResult<()>;
    fn unobserve_all(&self, id: ListenerId) -> ServiceResult<()>;
}

#[derive(Clone)]
pub struct GameServiceImpl {
    transport_service: ArcTransportService,
    player_connection_service: ArcPlayerConnectionService,
    player_service: ArcPlayerService,
    game_repository: ArcGameRepository,
    games: Arc<DashMap<GameId, Game>>,
    game_ended_tokens: Arc<DashMap<GameId, CancellationToken>>,
    game_spectators: Arc<ManyManyDashMap<GameId, ListenerId>>,
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

    async fn insert_empty_game(
        &self,
        white: &PlayerUsername,
        black: &PlayerUsername,
        seek: &Seek,
    ) -> ServiceResult<GameId> {
        let player_white = self.player_service.fetch_player_data(white).await?;
        let player_black = self.player_service.fetch_player_data(black).await?;
        let game_record = GameRecord {
            date: chrono::Utc::now(),
            white: white.clone(),
            black: black.clone(),
            rating_info: None,
            settings: seek.game_settings.clone(),
            game_type: seek.game_type.clone(),
            result: TakGameState::Ongoing,
            moves: vec![],
        };
        let game_id = self
            .game_repository
            .create_game(&game_record, &player_white, &player_black)
            .await?;
        Ok(game_id)
    }

    async fn save_to_database(&self, game: &Game) -> ServiceResult<()> {
        let update = GameResultUpdate {
            moves: game.game.action_history.clone(),
            result: game.game.base.game_state.clone(),
        };
        self.game_repository
            .update_game_result(game.id, &update)
            .await?;
        Ok(())
    }

    async fn check_game_over(&self, game_id: &GameId) {
        let game_ref = self.games.get(game_id);
        let game = match game_ref.as_ref() {
            Some(g) if !g.game.is_ongoing() => g.value().clone(),
            _ => return,
        };
        drop(game_ref);

        info!(
            "Game {} is over: {}",
            game.id,
            game_state_to_string(&game.game.base.game_state)
        );

        self.send_time_update(game_id).await;

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

        do_player_send(
            &self.player_connection_service,
            &self.transport_service,
            &game.white,
            &game_over_msg,
        )
        .await;

        do_player_send(
            &self.player_connection_service,
            &self.transport_service,
            &game.black,
            &game_over_msg,
        )
        .await;

        let spectators = self.game_spectators.remove_key(game_id);
        self.transport_service
            .try_listener_multicast(&spectators, &game_over_msg)
            .await;

        let game_remove_msg = ServerMessage::GameList {
            add: false,
            game: game.clone(),
        };
        do_player_broadcast(
            &self.player_connection_service,
            &self.transport_service,
            &game_remove_msg,
        )
        .await;

        if let Err(e) = self.save_to_database(&game).await {
            error!("Failed to save game to database: {}", e);
        }
    }

    async fn send_time_update(&self, game_id: &GameId) {
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
        do_player_send(
            &self.player_connection_service,
            &self.transport_service,
            &players.0,
            &time_update_msg,
        )
        .await;
        do_player_send(
            &self.player_connection_service,
            &self.transport_service,
            &players.1,
            &time_update_msg,
        )
        .await;

        let spectators = self.game_spectators.get_by_key(game_id);
        self.transport_service
            .try_listener_multicast(&spectators, &time_update_msg)
            .await;
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
                debug!(
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
            game_service.check_game_over(&game_id).await;
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

                debug!(
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
            game_service.check_game_over(&game_id).await;
        });
    }
}

#[async_trait::async_trait]
impl GameService for GameServiceImpl {
    fn observe_game(&self, id: ListenerId, game_id: &GameId) -> ServiceResult<()> {
        if self.games.get(game_id).is_none() {
            return ServiceError::not_found("Game ID not found");
        };

        self.game_spectators.insert(*game_id, id.clone());
        Ok(())
    }

    fn unobserve_game(&self, id: ListenerId, game_id: &GameId) -> ServiceResult<()> {
        self.game_spectators.remove(game_id, &id);
        Ok(())
    }

    async fn offer_draw(
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
            do_player_send(
                &self.player_connection_service,
                &self.transport_service,
                &opponent,
                &draw_offer_msg,
            )
            .await;
        }

        self.check_game_over(game_id).await;
        Ok(())
    }

    async fn request_undo(
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
            do_player_send(
                &self.player_connection_service,
                &self.transport_service,
                &opponent,
                &undo_request_msg,
            )
            .await;
        } else {
            self.send_time_update(game_id).await;

            let undo_msg = ServerMessage::GameMessage {
                game_id: *game_id,
                message: ServerGameMessage::Undo,
            };
            do_player_send(
                &self.player_connection_service,
                &self.transport_service,
                &username,
                &undo_msg,
            )
            .await;
            do_player_send(
                &self.player_connection_service,
                &self.transport_service,
                &opponent,
                &undo_msg,
            )
            .await;

            let spectators = self.game_spectators.get_by_key(game_id);
            self.transport_service
                .try_listener_multicast(&spectators, &undo_msg)
                .await;
        }

        Ok(())
    }

    async fn add_game_from_seek(
        &self,
        seek: &Seek,
        opponent: &PlayerUsername,
    ) -> ServiceResult<()> {
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

        let id = self.insert_empty_game(&white, &black, seek).await?;
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

        info!("Game {} created", id);

        let cancel_token = CancellationToken::new();
        self.game_ended_tokens.insert(id, cancel_token.clone());
        self.run_disconnect_waiter(id, cancel_token.clone());
        self.run_timeout_waiter(id, cancel_token);

        let game_new_msg = ServerMessage::GameList { add: true, game };
        do_player_broadcast(
            &self.player_connection_service,
            &self.transport_service,
            &game_new_msg,
        )
        .await;

        let game_start_msg = ServerMessage::GameStart { game_id: id };

        do_player_send(
            &self.player_connection_service,
            &self.transport_service,
            &seek.creator,
            &game_start_msg,
        )
        .await;
        do_player_send(
            &self.player_connection_service,
            &self.transport_service,
            &opponent,
            &game_start_msg,
        )
        .await;

        Ok(())
    }

    async fn try_do_action(
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
        let action_record = game_ref
            .game
            .do_action(&action)
            .map_err(|e| ServiceError::NotPossible(e))?;
        let opponent = Self::get_opponent_username(&game_ref, &player);
        drop(game_ref);

        self.send_time_update(game_id).await;

        let action_msg = ServerMessage::GameMessage {
            game_id: *game_id,
            message: ServerGameMessage::Action {
                action: action_record,
            },
        };

        do_player_send(
            &self.player_connection_service,
            &self.transport_service,
            &opponent,
            &action_msg,
        )
        .await;
        let spectators = self.game_spectators.get_by_key(game_id);
        self.transport_service
            .try_listener_multicast(&spectators, &action_msg)
            .await;

        self.check_game_over(game_id).await;
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

    async fn resign_game(&self, username: &PlayerUsername, game_id: &GameId) -> ServiceResult<()> {
        let Some(mut game_ref) = self.games.get_mut(game_id) else {
            return ServiceError::not_found("Game ID not found");
        };
        let player = Self::get_game_player(&game_ref, username)?;
        game_ref
            .game
            .resign(&player)
            .map_err(|e| ServiceError::NotPossible(e))?;
        drop(game_ref);

        self.check_game_over(game_id).await;
        Ok(())
    }

    fn unobserve_all(&self, id: ListenerId) -> ServiceResult<()> {
        self.game_spectators.remove_value(&id);
        Ok(())
    }
}

#[derive(Clone, Default)]
pub struct MockGameService {}

#[async_trait::async_trait]
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
    async fn add_game_from_seek(
        &self,
        _seek: &Seek,
        _opponent: &PlayerUsername,
    ) -> ServiceResult<()> {
        Ok(())
    }
    async fn try_do_action(
        &self,
        _username: &PlayerUsername,
        _game_id: &GameId,
        _action: TakAction,
    ) -> ServiceResult<()> {
        Ok(())
    }
    async fn resign_game(
        &self,
        _username: &PlayerUsername,
        _game_id: &GameId,
    ) -> ServiceResult<()> {
        Ok(())
    }
    async fn offer_draw(
        &self,
        _username: &PlayerUsername,
        _game_id: &GameId,
        _offer: bool,
    ) -> ServiceResult<()> {
        Ok(())
    }
    async fn request_undo(
        &self,
        _username: &PlayerUsername,
        _game_id: &GameId,
        _request: bool,
    ) -> ServiceResult<()> {
        Ok(())
    }
    fn observe_game(&self, _id: ListenerId, _game_id: &GameId) -> ServiceResult<()> {
        Ok(())
    }
    fn unobserve_game(&self, _id: ListenerId, _game_id: &GameId) -> ServiceResult<()> {
        Ok(())
    }
    fn unobserve_all(&self, _id: ListenerId) -> ServiceResult<()> {
        Ok(())
    }
}
