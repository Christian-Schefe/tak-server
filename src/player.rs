use std::sync::{Arc, LazyLock};

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

use crate::client::{ClientId, associate_player};

pub type PlayerUsername = String;

pub static PLAYER_DB_POOL: LazyLock<Pool<SqliteConnectionManager>> = LazyLock::new(|| {
    let db_path = std::env::var("TAK_PLAYER_DB").expect("TAK_PLAYER_DB env var not set");
    let manager = SqliteConnectionManager::file(db_path);
    Pool::builder()
        .max_size(5)
        .build(manager)
        .expect("Failed to create DB pool")
});

pub static PLAYER_CACHE: LazyLock<Arc<moka::sync::Cache<PlayerUsername, Player>>> =
    LazyLock::new(|| Arc::new(moka::sync::Cache::builder().max_capacity(1000).build()));

#[derive(Clone)]
pub struct Player {
    pub password_hash: String,
    pub is_bot: bool,
}

pub fn fetch_player(username: &str) -> Option<Player> {
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
