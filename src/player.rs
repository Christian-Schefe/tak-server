use dashmap::DashMap;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rand::{Rng, distr::Alphanumeric};
use std::{
    sync::{Arc, LazyLock},
    time::Duration,
};
use validator::Validate;

use crate::{
    client::{ClientId, associate_player, get_associated_player},
    email::send_email,
    jwt::validate_jwt,
};

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

static GUEST_PLAYER_TOKENS: LazyLock<Arc<moka::sync::Cache<String, PlayerUsername>>> =
    LazyLock::new(|| Arc::new(moka::sync::Cache::builder().time_to_idle(GUEST_TTL).build()));

static NEXT_GUEST_ID: LazyLock<Arc<std::sync::Mutex<u32>>> =
    LazyLock::new(|| Arc::new(std::sync::Mutex::new(1)));

static TAKEN_UNIQUE_USERNAMES: LazyLock<Arc<DashMap<PlayerUsername, ()>>> =
    LazyLock::new(|| Arc::new(DashMap::new()));

pub fn load_unique_usernames() -> Result<(), String> {
    let conn = PLAYER_DB_POOL
        .get()
        .map_err(|e| format!("Failed to get DB connection: {}", e))?;
    let mut stmt = conn
        .prepare("SELECT name FROM players")
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;
    let usernames = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| format!("Failed to query usernames: {}", e))?;
    for username in usernames {
        let username: String = username.map_err(|e| format!("Failed to get username: {}", e))?;
        let unique_username = uniquify_username(&username);
        TAKEN_UNIQUE_USERNAMES.insert(unique_username, ());
    }
    Ok(())
}

fn increment_guest_id() -> u32 {
    let mut id_lock = NEXT_GUEST_ID.lock().expect("Failed to lock guest ID mutex");
    let guest_id = *id_lock;
    *id_lock += 1;
    guest_id
}

#[derive(Clone)]
pub struct Player {
    pub id: i32,
    pub email: String,
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
                id: row.get("id")?,
                email: row.get("email")?,
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

pub fn validate_login(username: &PlayerUsername, password: &str) -> Result<(), String> {
    if let Some(player) = fetch_player(&username) {
        bcrypt::verify(password, &player.password_hash)
            .map_err(|_| "Invalid username or password".into())
            .and_then(|is_valid| {
                if is_valid {
                    Ok(())
                } else {
                    Err("Invalid username or password".into())
                }
            })
    } else {
        Err("Invalid username or password".into())
    }
}

pub fn try_login(id: &ClientId, username: &PlayerUsername, password: &str) -> Result<(), String> {
    validate_login(username, password)?;
    associate_player(id, username)
}

pub fn try_login_jwt(id: &ClientId, token: &str) -> Result<PlayerUsername, String> {
    let username = validate_jwt(token)?;
    associate_player(id, &username)?;
    Ok(username)
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

#[derive(Validate)]
struct EmailValidator {
    #[validate(email)]
    email: String,
}

pub fn try_register(username: &PlayerUsername, email: &str) -> Result<(), String> {
    if username.to_ascii_lowercase().starts_with("guest") {
        return Err("Username cannot start with 'Guest'".into());
    }
    if username.len() < 3 || username.len() > 15 {
        return Err("Username must be between 3 and 15 characters".into());
    }
    if username
        .chars()
        .next()
        .is_none_or(|c| !c.is_ascii_alphabetic())
    {
        return Err("Username must start with a letter".into());
    }
    if username
        .chars()
        .any(|c| !c.is_ascii_alphanumeric() && c != '_')
    {
        return Err("Username must be alphanumeric".into());
    }
    let email_validator = EmailValidator {
        email: email.to_string(),
    };
    if let Err(e) = email_validator.validate() {
        return Err(format!("Invalid email: {}", e));
    }
    try_take_username(username)?;
    create_player(username, email)
}

fn uniquify_username(username: &PlayerUsername) -> PlayerUsername {
    username
        .to_ascii_lowercase()
        .replace("_", "")
        .replace("i", "1")
        .replace("l", "1")
        .replace("o", "0")
}

fn try_take_username(username: &PlayerUsername) -> Result<(), String> {
    let unique_username = uniquify_username(username);
    if TAKEN_UNIQUE_USERNAMES.contains_key(&unique_username) {
        return Err("Username already taken".into());
    }
    TAKEN_UNIQUE_USERNAMES.insert(unique_username, ());
    Ok(())
}

fn create_player(username: &PlayerUsername, email: &str) -> Result<(), String> {
    let temp_password = generate_temporary_password();
    let password_hash = bcrypt::hash(&temp_password, bcrypt::DEFAULT_COST).unwrap();
    let conn = PLAYER_DB_POOL
        .get()
        .map_err(|e| format!("Failed to get DB connection: {}", e))?;
    let largest_player_id: i32 = conn
        .query_row("SELECT MAX(id) FROM players", [], |row| {
            row.get::<_, i32>(0)
        })
        .map_err(|e| format!("Failed to query largest player ID: {}", e))?;
    conn.execute(
        "INSERT INTO players (id, name, password, email) VALUES (?1, ?2, ?3, ?4)",
        (largest_player_id + 1, username, password_hash, email),
    )
    .map_err(|e| format!("Failed to insert player: {}", e))?;
    send_password_email(email, username, &temp_password, false)?;
    Ok(())
}

fn generate_temporary_password() -> String {
    let rng = rand::rng();
    rng.sample_iter(&Alphanumeric)
        .take(8)
        .map(char::from)
        .collect()
}

fn send_password_email(
    to: &str,
    username: &PlayerUsername,
    temp_password: &str,
    is_reset: bool,
) -> Result<(), String> {
    let subject = if is_reset {
        "Password Reset"
    } else {
        "Welcome to Playtak!"
    };
    let body = format!(
        "Hello {},\n\n\
        {}\n\n\
        Here are your login details:\n\
        Username: {}\n\
        Temporary Password: {}\n\n\
        Please log in and change your password as soon as possible.\n\n\
        Best regards,\n\
        The Playtak Team",
        username,
        if is_reset {
            "Your password has been reset successfully!"
        } else {
            "Your account has been created successfully!"
        },
        username,
        temp_password
    );
    send_email(to, &subject, &body)
}

pub fn reset_password(id: &ClientId) -> Result<(), String> {
    let Some(username) = get_associated_player(id) else {
        return Err("Client not associated with any player".into());
    };
    let player = fetch_player(&username).ok_or("Player not found")?;
    let temp_password = generate_temporary_password();
    let password_hash = bcrypt::hash(&temp_password, bcrypt::DEFAULT_COST).unwrap();

    let conn = PLAYER_DB_POOL
        .get()
        .map_err(|e| format!("Failed to get DB connection: {}", e))?;

    // Note the order of statements: email is sent before DB update to avoid locking out user if email fails.
    // Connection is established first to not unnecessarily send email if DB is unreachable.
    send_password_email(&player.email, &username, &temp_password, true)?;

    conn.execute(
        "UPDATE players SET password = ?1 WHERE id = ?2",
        (password_hash, player.id),
    )
    .map_err(|e| format!("Failed to update player password: {}", e))?;
    Ok(())
}
