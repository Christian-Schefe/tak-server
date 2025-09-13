use std::{
    sync::{Arc, LazyLock},
    time::Duration,
};

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

use crate::client::{ClientId, associate_player};

pub type PlayerUsername = String;

static PLAYER_DB_POOL: LazyLock<Pool<SqliteConnectionManager>> = LazyLock::new(|| {
    let db_path = std::env::var("TAK_PLAYER_DB").expect("TAK_PLAYER_DB env var not set");
    let manager = SqliteConnectionManager::file(db_path);
    Pool::builder()
        .max_size(5)
        .build(manager)
        .expect("Failed to create DB pool")
});

static PLAYER_CACHE: LazyLock<Arc<moka::sync::Cache<PlayerUsername, Player>>> =
    LazyLock::new(|| Arc::new(moka::sync::Cache::builder().max_capacity(1000).build()));

const GUEST_TTL: Duration = Duration::from_secs(60 * 60 * 4);

//TODO: Do we even need this?
static GUEST_PLAYER_TOKENS: LazyLock<Arc<moka::sync::Cache<String, PlayerUsername>>> =
    LazyLock::new(|| Arc::new(moka::sync::Cache::builder().time_to_idle(GUEST_TTL).build()));

static NEXT_GUEST_ID: LazyLock<Arc<std::sync::Mutex<u32>>> =
    LazyLock::new(|| Arc::new(std::sync::Mutex::new(1)));

fn increment_guest_id() -> u32 {
    let mut id_lock = NEXT_GUEST_ID.lock().expect("Failed to lock guest ID mutex");
    let guest_id = *id_lock;
    *id_lock += 1;
    guest_id
}

#[derive(Clone)]
pub struct Player {
    pub password_hash: String,
    pub is_bot: bool,
}

pub fn fetch_player(username: &str) -> Option<Player> {
    if username.starts_with("Guest") {
        return None;
    }
    let username = username.to_string();
    let cache = PLAYER_CACHE.clone();
    if let Some(player) = cache.get(&username) {
        return Some(player);
    }
    let player = PLAYER_DB_POOL.get().ok()?.query_one(
        "SELECT * FROM players WHERE name = ?1",
        [username.clone()],
        |row| {
            Ok(Player {
                password_hash: row.get("password")?,
                is_bot: row.get::<_, i32>("isbot")? != 0,
            })
        },
    );
    match player {
        Ok(p) => {
            cache.insert(username.clone(), p.clone());
            Some(p)
        }
        Err(_) => None,
    }
}

pub fn validate_login(username: &PlayerUsername, password: &str) -> bool {
    if let Some(player) = fetch_player(&username) {
        bcrypt::verify(password, &player.password_hash).unwrap_or(false)
    } else {
        false
    }
}

pub fn try_login(id: &ClientId, username: &PlayerUsername, password: &str) -> bool {
    if !validate_login(username, password) {
        return false;
    }
    associate_player(id, username).is_ok()
}

pub fn login_guest(id: &ClientId, token: Option<&str>) {
    let guest_name = token
        .and_then(|x| GUEST_PLAYER_TOKENS.get(x))
        .unwrap_or_else(|| format!("Guest{}", increment_guest_id()));

    if let Err(e) = associate_player(id, &guest_name) {
        eprintln!("Failed to login guest player: {}", e);
    } else if let Some(token) = token {
        GUEST_PLAYER_TOKENS.insert(guest_name.clone(), token.to_string());
    }
}
