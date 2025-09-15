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
    client::{
        ClientId, get_associated_client, try_auth_protocol_broadcast, try_protocol_multicast,
        try_protocol_send,
    },
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

static GAMES: LazyLock<Arc<DashMap<GameId, Game>>> = LazyLock::new(|| Arc::new(DashMap::new()));

static GAME_TIMEOUT_TOKENS: LazyLock<Arc<DashMap<GameId, CancellationToken>>> =
    LazyLock::new(|| Arc::new(DashMap::new()));

static GAME_SPECTATORS: LazyLock<Arc<DashMap<GameId, Vec<ClientId>>>> =
    LazyLock::new(|| Arc::new(DashMap::new()));

static GAME_BY_PLAYER: LazyLock<Arc<DashMap<PlayerUsername, GameId>>> =
    LazyLock::new(|| Arc::new(DashMap::new()));

fn get_game_player(game: &Game, username: &PlayerUsername) -> Result<TakPlayer, String> {
    if &game.white == username {
        Ok(TakPlayer::White)
    } else if &game.black == username {
        Ok(TakPlayer::Black)
    } else {
        Err("You are not a player in this game".to_string())
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
) -> Result<GameId, String> {
    let conn = GAMES_DB_POOL
        .get()
        .map_err(|_| "Failed to get DB connection")?;
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
    .map_err(|_| "Failed to insert empty game")?;
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
        .map(move_to_database_string)
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

pub fn add_game_from_seek(seek: &Seek, opponent: &PlayerUsername) -> Result<(), String> {
    if &seek.creator == opponent {
        return Err("You cannot accept your own seek".into());
    }
    if has_active_game(&seek.creator) {
        return Err("Player is already in a game".into());
    }
    if has_active_game(opponent) {
        return Err("Player is already in a game".into());
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
    let id = insert_empty_game(&white, &black, seek)?;
    let game = Game {
        id,
        white,
        black,
        game: TakGame::new(seek.game_settings.clone()),
        game_type: seek.game_type.clone(),
    };
    GAMES.insert(id, game.clone());
    GAME_BY_PLAYER.insert(seek.creator.clone(), id);
    GAME_BY_PLAYER.insert(opponent.clone(), id);

    println!("Game {} created", id);

    let cancel_token = CancellationToken::new();
    GAME_TIMEOUT_TOKENS.insert(id, cancel_token.clone());
    run_timeout_waiter(id, cancel_token);

    let game_new_msg = ServerMessage::GameList {
        add: true,
        game: game.clone(),
    };
    try_auth_protocol_broadcast(&game_new_msg);

    let game_start_msg = ServerMessage::GameStart { game };
    get_associated_client(&seek.creator).map(|id| try_protocol_send(&id, &game_start_msg));
    get_associated_client(&opponent).map(|id| try_protocol_send(&id, &game_start_msg));
    Ok(())
}

pub fn try_do_action(
    username: &PlayerUsername,
    game_id: &GameId,
    action: TakAction,
) -> Result<(), String> {
    let mut game_ref = GAMES
        .get_mut(game_id)
        .ok_or_else(|| "Game ID not found".to_string())?;
    let player = get_game_player(&game_ref, username)?;
    if game_ref.game.base.current_player != player {
        return Err("It's not your turn".to_string());
    }
    game_ref.game.do_action(&action)?;
    let opponent = get_opponent_username(&game_ref, &player);
    drop(game_ref);

    send_time_update(game_id);

    let action_msg = ServerMessage::GameMessage {
        game_id: *game_id,
        message: ServerGameMessage::Action(action.clone()),
    };

    get_associated_client(&opponent).map(|id| try_protocol_send(&id, &action_msg));
    if let Some(spectators) = GAME_SPECTATORS.get(game_id) {
        try_protocol_multicast(&spectators.value(), &action_msg);
    }

    check_game_over(game_id);
    Ok(())
}

pub fn get_games() -> Vec<Game> {
    GAMES.iter().map(|entry| entry.value().clone()).collect()
}

pub fn has_active_game(player: &PlayerUsername) -> bool {
    GAME_BY_PLAYER.contains_key(player)
}

pub fn get_active_game_of_player(player: &PlayerUsername) -> Option<Game> {
    for entry in GAMES.iter() {
        let game = entry.value();
        if &game.white == player || &game.black == player {
            return Some(game.clone());
        }
    }
    None
}

pub fn resign_game(username: &PlayerUsername, game_id: &GameId) -> Result<(), String> {
    let mut game_ref = GAMES
        .get_mut(game_id)
        .ok_or_else(|| "Game ID not found".to_string())?;
    let player = get_game_player(&game_ref, username)?;
    game_ref.game.resign(&player)?;
    drop(game_ref);

    check_game_over(game_id);
    Ok(())
}

fn check_game_over(game_id: &GameId) {
    let game_ref = GAMES.get(game_id);
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

    GAMES.remove(game_id);
    if let Some((_, token)) = GAME_TIMEOUT_TOKENS.remove(game_id) {
        token.cancel();
    }

    let game_over_msg = ServerMessage::GameMessage {
        game_id: *game_id,
        message: ServerGameMessage::GameOver(game.game.base.game_state.clone()),
    };
    get_associated_client(&game.white).map(|id| try_protocol_send(&id, &game_over_msg));
    get_associated_client(&game.black).map(|id| try_protocol_send(&id, &game_over_msg));

    if let Some((_, spectators)) = GAME_SPECTATORS.remove(game_id) {
        try_protocol_multicast(&spectators, &game_over_msg);
    }

    let game_remove_msg = ServerMessage::GameList {
        add: false,
        game: game.clone(),
    };
    try_auth_protocol_broadcast(&game_remove_msg);

    if let Err(e) = save_to_database(&game) {
        eprintln!("Failed to save game to database: {}", e);
    }
}

pub fn observe_game(id: &ClientId, game_id: &GameId) -> Result<(), String> {
    let game_ref = GAMES
        .get(game_id)
        .ok_or_else(|| "Game ID not found".to_string())?;
    let game = game_ref.value().clone();
    drop(game_ref);

    let mut spectators = GAME_SPECTATORS
        .entry(*game_id)
        .or_insert_with(|| Vec::new());
    if !spectators.contains(&id) {
        spectators.push(id.clone());
    }
    drop(spectators);

    let msg = ServerMessage::ObserveGame { game };
    try_protocol_send(id, &msg);

    Ok(())
}

pub fn unobserve_game(id: &ClientId, game_id: &GameId) -> Result<(), String> {
    if let Some(mut spectators) = GAME_SPECTATORS.get_mut(game_id) {
        spectators.retain(|u| u != id);
    }
    Ok(())
}

pub fn offer_draw(username: &PlayerUsername, game_id: &GameId, offer: bool) -> Result<(), String> {
    let mut game_ref = GAMES
        .get_mut(game_id)
        .ok_or_else(|| "Game ID not found".to_string())?;
    let player = get_game_player(&game_ref, username)?;
    let did_draw = game_ref.game.offer_draw(&player, offer)?;
    let opponent = get_opponent_username(&game_ref, &player);
    drop(game_ref);

    if !did_draw {
        let draw_offer_msg = ServerMessage::GameMessage {
            game_id: *game_id,
            message: ServerGameMessage::DrawOffer { offer },
        };
        get_associated_client(&opponent).map(|id| try_protocol_send(&id, &draw_offer_msg));
    }

    check_game_over(game_id);
    Ok(())
}

pub fn request_undo(
    username: &PlayerUsername,
    game_id: &GameId,
    request: bool,
) -> Result<(), String> {
    let mut game_ref = GAMES
        .get_mut(game_id)
        .ok_or_else(|| "Game ID not found".to_string())?;
    let player = get_game_player(&game_ref, username)?;
    let did_undo = game_ref.game.request_undo(&player, request)?;
    let opponent = get_opponent_username(&game_ref, &player);
    drop(game_ref);

    if !did_undo {
        let undo_request_msg = ServerMessage::GameMessage {
            game_id: *game_id,
            message: ServerGameMessage::UndoRequest { request },
        };
        get_associated_client(&opponent).map(|id| try_protocol_send(&id, &undo_request_msg));
    } else {
        let undo_msg = ServerMessage::GameMessage {
            game_id: *game_id,
            message: ServerGameMessage::Undo,
        };
        get_associated_client(&username).map(|id| try_protocol_send(&id, &undo_msg));
        get_associated_client(&opponent).map(|id| try_protocol_send(&id, &undo_msg));

        if let Some(spectators) = GAME_SPECTATORS.get(game_id) {
            try_protocol_multicast(&spectators, &undo_msg);
        }
    }

    Ok(())
}

fn send_time_update(game_id: &GameId) {
    let game_ref = GAMES.get(game_id);
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
    get_associated_client(&players.0).map(|id| try_protocol_send(&id, &time_update_msg));
    get_associated_client(&players.1).map(|id| try_protocol_send(&id, &time_update_msg));
    if let Some(spectators) = GAME_SPECTATORS.get(game_id) {
        try_protocol_multicast(&spectators.value(), &time_update_msg);
    }
}

pub fn run_timeout_waiter(game_id: GameId, cancel_token: CancellationToken) {
    tokio::spawn(async move {
        loop {
            let Some(mut game_ref) = GAMES.get_mut(&game_id) else {
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
        GAME_TIMEOUT_TOKENS.remove(&game_id);
        check_game_over(&game_id);
    });
}
