use std::sync::{Arc, LazyLock};

use dashmap::DashMap;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

use crate::{
    client::{get_associated_client, get_protocol_handler, try_protocol_broadcast},
    player::PlayerUsername,
    seek::{GameType, Seek},
    tak::{TakAction, TakGame, TakGameState, TakPlayer},
};

pub static GAMES_DB_POOL: LazyLock<Pool<SqliteConnectionManager>> = LazyLock::new(|| {
    let db_path = std::env::var("TAK_GAMES_DB").expect("TAK_GAMES_DB env var not set");
    let manager = SqliteConnectionManager::file(db_path);
    Pool::builder()
        .max_size(5)
        .build(manager)
        .expect("Failed to create DB pool")
});

pub type GameId = u32;

#[derive(Clone)]
pub struct Game {
    pub id: GameId,
    pub white: PlayerUsername,
    pub black: PlayerUsername,
    pub game: TakGame,
    pub game_type: GameType,
}

static GAMES: LazyLock<Arc<DashMap<GameId, Game>>> = LazyLock::new(|| Arc::new(DashMap::new()));

fn insert_empty_game(
    white: &PlayerUsername,
    black: &PlayerUsername,
    seek: &Seek,
) -> Result<GameId, String> {
    let conn = GAMES_DB_POOL
        .get()
        .map_err(|_| "Failed to get DB connection")?;
    conn.execute(
        "INSERT INTO games (date, size, player_white, player_black, timertime, timerinc, notation, result, rating_white, rating_black, unrated, tournament, komi, pieces, capstones, rating_change_white, rating_change_black, extra_time_amount, extra_time_trigger)  VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
        [
            chrono::Utc::now().naive_utc().to_string(),
            seek.game_settings.board_size.to_string(),
            white.to_string(),
            black.to_string(),
            seek.game_settings.time_contingent_seconds.to_string(),
            seek.game_settings.time_increment_seconds.to_string(),
            "".to_string(),
            "0-0".to_string(),
            "0".to_string(),
            "0".to_string(),
            if seek.game_type == crate::seek::GameType::Unrated { "1" } else { "0" }.to_string(),
            if seek.game_type == crate::seek::GameType::Tournament { "1" } else { "0" }.to_string(),
            seek.game_settings.half_komi.to_string(),
            seek.game_settings.reserve_pieces.to_string(),
            seek.game_settings.reserve_capstones.to_string(),
            "0".to_string(),
            "0".to_string(),
            seek.game_settings.time_extra.as_ref().map_or("0".to_string(), |x| x.extra_seconds.to_string()),
            seek.game_settings.time_extra.as_ref().map_or("0".to_string(), |x| x.trigger_move.to_string()),
        ]
    )
    .map_err(|_| "Failed to insert empty game")?;
    Ok(conn.last_insert_rowid() as GameId)
}

pub fn add_game_from_seek(seek: &Seek, opponent: &PlayerUsername) -> Result<(), String> {
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

    try_protocol_broadcast(|handler| handler.send_new_game_message(&game));

    get_associated_client(&seek.creator)
        .map(|id| get_protocol_handler(&id).send_game_start_message(&game));
    get_associated_client(&opponent)
        .map(|id| get_protocol_handler(&id).send_game_start_message(&game));
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
    let player = if &game_ref.white == username {
        TakPlayer::White
    } else if &game_ref.black == username {
        TakPlayer::Black
    } else {
        return Err("You are not a player in this game".to_string());
    };
    if game_ref.game.current_player != player {
        return Err("It's not your turn".to_string());
    }
    game_ref.game.do_action(&action)?;
    let opponent = if player == TakPlayer::White {
        &game_ref.black
    } else {
        &game_ref.white
    };
    get_associated_client(opponent)
        .map(|id| get_protocol_handler(&id).send_game_action_message(&game_ref, &action));
    drop(game_ref);

    check_game_over(game_id);
    Ok(())
}

pub fn resign_game(username: &PlayerUsername, game_id: &GameId) -> Result<(), String> {
    let mut game_ref = GAMES
        .get_mut(game_id)
        .ok_or_else(|| "Game ID not found".to_string())?;
    let player = if &game_ref.white == username {
        TakPlayer::White
    } else if &game_ref.black == username {
        TakPlayer::Black
    } else {
        return Err("You are not a player in this game".to_string());
    };
    game_ref.game.resign(&player);
    drop(game_ref);

    check_game_over(game_id);
    Ok(())
}

fn check_game_over(game_id: &GameId) {
    let game_ref = GAMES.get(game_id);
    let game = match game_ref.as_ref() {
        Some(g) if g.game.game_state == TakGameState::Ongoing => g.value().clone(),
        _ => return,
    };
    drop(game_ref);

    get_associated_client(&game.white)
        .map(|id| get_protocol_handler(&id).send_game_over_message(&game));
    get_associated_client(&game.black)
        .map(|id| get_protocol_handler(&id).send_game_over_message(&game));

    try_protocol_broadcast(|handler| {
        handler.send_remove_game_message(&game);
    });
    GAMES.remove(game_id);
}
