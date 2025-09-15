use std::sync::{Arc, LazyLock};

use dashmap::DashMap;

use crate::{
    client::try_auth_protocol_broadcast,
    game::add_game_from_seek,
    player::PlayerUsername,
    protocol::ServerMessage,
    tak::{TakGameSettings, TakPlayer},
};

#[derive(Clone, Debug)]
pub struct Seek {
    pub id: SeekId,
    pub creator: PlayerUsername,
    pub opponent: Option<PlayerUsername>,
    pub color: Option<TakPlayer>,
    pub game_settings: TakGameSettings,
    pub game_type: GameType,
}

pub type SeekId = u32;

#[derive(Clone, Debug, PartialEq)]
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
    if !game_settings.is_valid() {
        return Err("Invalid game settings".into());
    }
    if SEEKS_BY_PLAYER.contains_key(&player) {
        remove_seek_of_player(&player)?;
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

    let seek_new_msg = ServerMessage::SeekList {
        add: true,
        seek: seek.clone(),
    };
    try_auth_protocol_broadcast(&seek_new_msg);

    Ok(())
}

pub fn get_seeks() -> Vec<Seek> {
    SEEKS.iter().map(|entry| entry.value().clone()).collect()
}

pub fn remove_seek_of_player(player: &PlayerUsername) -> Result<Seek, String> {
    let Some((_, seek_id)) = SEEKS_BY_PLAYER.remove(player) else {
        return Err("Player has no active seek".into());
    };
    let Some((_, seek)) = SEEKS.remove(&seek_id) else {
        return Err("Seek ID not found".into());
    };

    let seek_remove_msg = ServerMessage::SeekList {
        add: false,
        seek: seek.clone(),
    };
    try_auth_protocol_broadcast(&seek_remove_msg);

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
    add_game_from_seek(&seek, &player)?;
    let _ = remove_seek_of_player(&seek.creator);
    let _ = remove_seek_of_player(&player);
    Ok(())
}
