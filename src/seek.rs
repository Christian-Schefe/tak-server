use std::sync::{Arc, LazyLock};

use dashmap::DashMap;

use crate::{
    client::{ClientId, get_protocol_handler, try_protocol_broadcast},
    game::add_game_from_seek,
    player::PlayerUsername,
    tak::{TakGameSettings, TakPlayer},
};

#[derive(Clone)]
pub struct Seek {
    pub id: SeekId,
    pub creator: PlayerUsername,
    pub opponent: Option<PlayerUsername>,
    pub color: Option<TakPlayer>,
    pub game_settings: TakGameSettings,
    pub game_type: GameType,
}

pub type SeekId = u32;

#[derive(Clone, PartialEq)]
pub enum GameType {
    Unrated,
    Rated,
    Tournament,
}

static SEEKS: LazyLock<Arc<DashMap<SeekId, Seek>>> = LazyLock::new(|| Arc::new(DashMap::new()));
static SEEKS_BY_PLAYER: LazyLock<Arc<DashMap<PlayerUsername, SeekId>>> =
    LazyLock::new(|| Arc::new(DashMap::new()));

static NEXT_SEEK_ID: LazyLock<Arc<std::sync::Mutex<SeekId>>> =
    LazyLock::new(|| Arc::new(std::sync::Mutex::new(1)));

fn increment_seek_id() -> SeekId {
    let mut id_lock = NEXT_SEEK_ID.lock().expect("Failed to lock seek ID mutex");
    let seek_id = *id_lock;
    *id_lock += 1;
    seek_id
}

pub fn add_seek(
    player: PlayerUsername,
    opponent: Option<PlayerUsername>,
    color: Option<TakPlayer>,
    game_settings: TakGameSettings,
    game_type: GameType,
) -> Result<(), String> {
    if SEEKS_BY_PLAYER.contains_key(&player) {
        remove_seek_of_player(&player)?;
    }
    if !game_settings.is_valid() {
        println!("Player removed his seek due to invalid settings");
        return Ok(());
    }
    let seek_id = increment_seek_id();
    let seek = Seek {
        creator: player.clone(),
        id: seek_id,
        opponent,
        color,
        game_settings,
        game_type,
    };

    let seek_id = seek.id;
    SEEKS.insert(seek_id, seek.clone());
    SEEKS_BY_PLAYER.insert(player, seek_id);

    try_protocol_broadcast(|handler| handler.send_new_seek_message(&seek));

    Ok(())
}

pub fn send_seeks_to(id: &ClientId) {
    for entry in SEEKS.iter() {
        let seek = entry.value();
        get_protocol_handler(id).send_new_seek_message(&seek);
    }
}

pub fn remove_seek_of_player(player: &PlayerUsername) -> Result<Seek, String> {
    let Some((_, seek_id)) = SEEKS_BY_PLAYER.remove(player) else {
        return Err("Player has no active seek".into());
    };
    let Some((_, seek)) = SEEKS.remove(&seek_id) else {
        return Err("Seek ID not found".into());
    };

    try_protocol_broadcast(|handler| handler.send_remove_seek_message(&seek));

    Ok(seek)
}

pub fn accept_seek(id: &SeekId, player: &PlayerUsername) -> Result<(), String> {
    let seek_ref = SEEKS
        .get(id)
        .ok_or_else(|| "Seek ID not found".to_string())?;
    let seek = seek_ref.value().clone();
    drop(seek_ref);
    if let Some(ref opponent) = seek.opponent {
        if opponent != player {
            return Err("This seek is not for you".into());
        }
    }
    remove_seek_of_player(&seek.creator)?;
    let _ = remove_seek_of_player(&player);
    add_game_from_seek(&seek, &player)?;
    Ok(())
}
