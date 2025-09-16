use dashmap::DashMap;
use passwords::PasswordGenerator;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rustrict::CensorStr;
use std::{
    sync::{Arc, LazyLock},
    time::{Duration, Instant},
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

const PASSWORD_RESET_TOKEN_TTL: Duration = Duration::from_secs(60 * 60 * 24);

static PASSWORD_RESET_TOKENS: LazyLock<Arc<moka::sync::Cache<String, (PlayerUsername, Instant)>>> =
    LazyLock::new(|| {
        Arc::new(
            moka::sync::Cache::builder()
                .time_to_live(PASSWORD_RESET_TOKEN_TTL)
                .build(),
        )
    });

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
    pub is_gagged: bool,
    pub access_level: AccessLevel,
}

#[derive(Clone, PartialEq)]
pub enum AccessLevel {
    User,
    Mod,
    Admin,
}

pub fn fetch_player(username: &str) -> Option<Player> {
    if username.starts_with("Guest") {
        return None;
    }
    let username = username.to_string();
    if let Some(player) = PLAYER_CACHE.get(&username) {
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
                is_gagged: row.get::<_, i32>("is_gagged")? != 0,
                access_level: if row.get::<_, i32>("is_admin")? != 0 {
                    AccessLevel::Admin
                } else if row.get::<_, i32>("is_mod")? != 0 {
                    AccessLevel::Mod
                } else {
                    AccessLevel::User
                },
            })
        },
    );
    match player {
        Ok(p) => {
            PLAYER_CACHE.insert(username.clone(), p.clone());
            Some(p)
        }
        Err(_) => None,
    }
}

fn more_rights(this: &Player, target: &Player) -> bool {
    match this.access_level {
        AccessLevel::Admin => target.access_level != AccessLevel::Admin,
        AccessLevel::Mod => target.access_level == AccessLevel::User,
        AccessLevel::User => false,
    }
}

fn more_rights_and_admin(this: &Player, target: &Player) -> bool {
    this.access_level == AccessLevel::Admin && target.access_level != AccessLevel::Admin
}

pub fn set_gagged(id: &ClientId, username: &PlayerUsername, gagged: bool) -> Result<(), String> {
    update_player(id, username, more_rights, |player, conn| {
        if player.access_level != AccessLevel::User {
            return Err(rusqlite::Error::InvalidQuery);
        }
        conn.execute(
            "UPDATE players SET is_gagged = ?1 WHERE id = ?2",
            (gagged as i32, player.id),
        )
    })
}

pub fn set_banned(id: &ClientId, username: &PlayerUsername, banned: bool) -> Result<(), String> {
    update_player(id, username, more_rights, |player, conn| {
        conn.execute(
            "UPDATE players SET is_banned = ?1 WHERE id = ?2",
            (banned as i32, player.id),
        )
    })
}

pub fn set_modded(id: &ClientId, username: &PlayerUsername, modded: bool) -> Result<(), String> {
    update_player(id, username, more_rights_and_admin, |player, conn| {
        conn.execute(
            "UPDATE players SET is_mod = ?1 WHERE id = ?2",
            (modded as i32, player.id),
        )
    })
}

pub fn set_admin(id: &ClientId, username: &PlayerUsername, admin: bool) -> Result<(), String> {
    update_player(id, username, more_rights_and_admin, |player, conn| {
        conn.execute(
            "UPDATE players SET is_admin = ?1 WHERE id = ?2",
            (admin as i32, player.id),
        )
    })
}

pub fn set_bot(id: &ClientId, username: &PlayerUsername, bot: bool) -> Result<(), String> {
    update_player(id, username, more_rights_and_admin, |player, conn| {
        conn.execute(
            "UPDATE players SET isbot = ?1 WHERE id = ?2",
            (bot as i32, player.id),
        )
    })
}

fn update_player(
    id: &ClientId,
    username: &PlayerUsername,
    access_predicate: impl Fn(&Player, &Player) -> bool,
    database_update_fn: impl Fn(
        &Player,
        &PooledConnection<SqliteConnectionManager>,
    ) -> Result<usize, rusqlite::Error>,
) -> Result<(), String> {
    let Some(current_username) = get_associated_player(id) else {
        return Err("Not logged in".into());
    };
    let Some(current_player) = fetch_player(&current_username) else {
        return Err("Current player not found".into());
    };
    let player = fetch_player(username).ok_or("Player not found")?;
    if !access_predicate(&current_player, &player) {
        return Err("Insufficient access level".into());
    }
    let conn = PLAYER_DB_POOL
        .get()
        .map_err(|e| format!("Failed to get DB connection: {}", e))?;
    database_update_fn(&player, &conn).map_err(|e| format!("Failed to update player: {}", e))?;
    PLAYER_CACHE.invalidate(username);
    Ok(())
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

pub fn try_login_guest(id: &ClientId, token: Option<&str>) -> Result<PlayerUsername, String> {
    let guest_name = token
        .and_then(|x| GUEST_PLAYER_TOKENS.get(x))
        .unwrap_or_else(|| format!("Guest{}", increment_guest_id()));

    associate_player(id, &guest_name)?;
    if let Some(token) = token {
        GUEST_PLAYER_TOKENS.insert(guest_name.clone(), token.to_string());
    }
    Ok(guest_name)
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
    if username.is_inappropriate() {
        return Err("Username contains inappropriate content".into());
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
    send_password_email(email, username, &temp_password)?;
    Ok(())
}

fn generate_temporary_password() -> String {
    let password_gen = PasswordGenerator::new()
        .length(8)
        .numbers(true)
        .lowercase_letters(true)
        .uppercase_letters(false)
        .spaces(false)
        .symbols(false)
        .exclude_similar_characters(true)
        .strict(true);
    password_gen.generate_one().unwrap()
}

fn send_password_email(
    to: &str,
    username: &PlayerUsername,
    temp_password: &str,
) -> Result<(), String> {
    let subject = "Welcome to Playtak!";
    let body = format!(
        "Hello {},\n\n\
        Your account has been created successfully!\n\n\
        Here are your login details:\n\
        Username: {}\n\
        Temporary Password: {}\n\n\
        Please log in and change your password as soon as possible.\n\n\
        Best regards,\n\
        The Playtak Team",
        username, username, temp_password
    );
    send_email(to, &subject, &body)
}

fn send_reset_token_email(
    to: &str,
    username: &PlayerUsername,
    reset_token: &str,
) -> Result<(), String> {
    let subject = "Playtak Password Reset Request";
    let body = format!(
        "Hello {},\n\n\
        To reset your password, please use the following token:\n\
        Reset Token: {}\n\n\
        This token is valid for 24 hours.\n\n\
        If you did not request a password reset, please ignore this email.\n\n\
        Best regards,\n\
        The Playtak Team",
        username, reset_token
    );
    send_email(to, &subject, &body)
}

pub fn send_reset_token(username: &PlayerUsername, email: &str) -> Result<(), String> {
    let player = fetch_player(username).ok_or("Player not found")?;
    if player.email != email {
        return Err("Email does not match".into());
    }
    let reset_token = generate_temporary_password();
    PASSWORD_RESET_TOKENS.insert(reset_token.clone(), (username.clone(), Instant::now()));
    send_reset_token_email(email, username, &reset_token)?;
    Ok(())
}

pub fn reset_password(
    username: &PlayerUsername,
    reset_token: &str,
    new_password: &str,
) -> Result<(), String> {
    let player = fetch_player(username).ok_or("Player not found")?;

    let (token_username, token_time) = PASSWORD_RESET_TOKENS
        .remove(reset_token)
        .ok_or("Invalid or expired reset token")?;
    if &token_username != username {
        return Err("Invalid or expired reset token for this user".into());
    }
    if token_time.elapsed() > PASSWORD_RESET_TOKEN_TTL {
        return Err("Invalid or expired reset token for this user".into());
    }

    let password_hash = bcrypt::hash(new_password, bcrypt::DEFAULT_COST)
        .map_err(|e| format!("Failed to hash password: {}", e))?;

    let conn = PLAYER_DB_POOL
        .get()
        .map_err(|e| format!("Failed to get DB connection: {}", e))?;

    conn.execute(
        "UPDATE players SET password = ?1 WHERE id = ?2",
        (password_hash, player.id),
    )
    .map_err(|e| format!("Failed to update player password: {}", e))?;
    PLAYER_CACHE.invalidate(username);
    Ok(())
}

pub fn change_password(
    id: &ClientId,
    current_password: &str,
    new_password: &str,
) -> Result<(), String> {
    let username = get_associated_player(id).ok_or("Not logged in")?;
    validate_login(&username, current_password)?;

    let player = fetch_player(&username).ok_or("Player not found")?;
    let password_hash = bcrypt::hash(new_password, bcrypt::DEFAULT_COST)
        .map_err(|e| format!("Failed to hash password: {}", e))?;

    let conn = PLAYER_DB_POOL
        .get()
        .map_err(|e| format!("Failed to get DB connection: {}", e))?;

    conn.execute(
        "UPDATE players SET password = ?1 WHERE id = ?2",
        (password_hash, player.id),
    )
    .map_err(|e| format!("Failed to update player password: {}", e))?;
    PLAYER_CACHE.invalidate(&username);
    Ok(())
}
