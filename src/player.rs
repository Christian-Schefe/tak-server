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
    DatabaseError, ServiceError, ServiceResult,
    client::{ClientId, ClientService},
    email::EmailService,
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

const GUEST_TTL: Duration = Duration::from_secs(60 * 60 * 4);

const PASSWORD_RESET_TOKEN_TTL: Duration = Duration::from_secs(60 * 60 * 24);

#[derive(Validate)]
struct EmailValidator {
    #[validate(email)]
    email: String,
}

#[derive(Clone)]
pub struct Player {
    pub id: i32,
    pub username: PlayerUsername,
    pub email: String,
    pub rating: i32,
    pub password_hash: String,
    pub is_bot: bool,
    pub is_gagged: bool,
    pub is_mod: bool,
    pub is_admin: bool,
}

pub trait PlayerService {
    fn load_unique_usernames(&self) -> ServiceResult<()>;
    fn fetch_player(&self, username: &str) -> Option<Player>;
    fn validate_login(&self, username: &PlayerUsername, password: &str) -> ServiceResult<()>;
    fn try_login(
        &self,
        id: &ClientId,
        username: &PlayerUsername,
        password: &str,
    ) -> ServiceResult<()>;
    fn try_login_jwt(&self, id: &ClientId, token: &str) -> ServiceResult<PlayerUsername>;
    fn try_login_guest(&self, id: &ClientId, token: Option<&str>) -> ServiceResult<PlayerUsername>;
    fn try_register(&self, username: &PlayerUsername, email: &str) -> ServiceResult<()>;
    fn send_reset_token(&self, username: &PlayerUsername, email: &str) -> ServiceResult<()>;
    fn reset_password(
        &self,
        username: &PlayerUsername,
        reset_token: &str,
        new_password: &str,
    ) -> ServiceResult<()>;
    fn change_password(
        &self,
        username: &PlayerUsername,
        current_password: &str,
        new_password: &str,
    ) -> ServiceResult<()>;
    fn set_gagged(
        &self,
        username: &PlayerUsername,
        target_username: &PlayerUsername,
        gagged: bool,
    ) -> ServiceResult<()>;
    fn set_banned(
        &self,
        id: &ClientId,
        username: &PlayerUsername,
        target_username: &PlayerUsername,
        banned: Option<String>,
    ) -> ServiceResult<()>;
    fn set_modded(
        &self,
        username: &PlayerUsername,
        target_username: &PlayerUsername,
        modded: bool,
    ) -> ServiceResult<()>;
    fn set_admin(
        &self,
        username: &PlayerUsername,
        target_username: &PlayerUsername,
        admin: bool,
    ) -> ServiceResult<()>;
    fn set_bot(
        &self,
        username: &PlayerUsername,
        target_username: &PlayerUsername,
        bot: bool,
    ) -> ServiceResult<()>;
    fn get_players(
        &self,
        ban_filter: Option<bool>,
        gag_filter: Option<bool>,
        mod_filter: Option<bool>,
        admin_filter: Option<bool>,
        bot_filter: Option<bool>,
    ) -> ServiceResult<Vec<Player>>;
    fn set_password(
        &self,
        username: &PlayerUsername,
        target_username: &PlayerUsername,
        new_password: &str,
    ) -> ServiceResult<()>;
}

pub struct PlayerServiceImpl {
    client_service: Arc<Box<dyn ClientService + Send + Sync>>,
    email_service: Arc<Box<dyn EmailService + Send + Sync>>,
    player_cache: Arc<moka::sync::Cache<PlayerUsername, Player>>,
    guest_player_tokens: Arc<moka::sync::Cache<String, PlayerUsername>>,
    next_guest_id: Arc<std::sync::Mutex<u32>>,
    taken_unique_usernames: Arc<DashMap<PlayerUsername, ()>>,
    password_reset_tokens: Arc<moka::sync::Cache<String, (PlayerUsername, Instant)>>,
}

impl PlayerServiceImpl {
    pub fn new(
        client_service: Arc<Box<dyn ClientService + Send + Sync>>,
        email_service: Arc<Box<dyn EmailService + Send + Sync>>,
    ) -> Self {
        Self {
            client_service,
            email_service,
            player_cache: Arc::new(moka::sync::Cache::builder().max_capacity(1000).build()),
            guest_player_tokens: Arc::new(
                moka::sync::Cache::builder().time_to_idle(GUEST_TTL).build(),
            ),
            next_guest_id: Arc::new(std::sync::Mutex::new(1)),
            taken_unique_usernames: Arc::new(DashMap::new()),
            password_reset_tokens: Arc::new(
                moka::sync::Cache::builder()
                    .time_to_live(PASSWORD_RESET_TOKEN_TTL)
                    .build(),
            ),
        }
    }

    fn increment_guest_id(&self) -> u32 {
        let mut id_lock = self
            .next_guest_id
            .lock()
            .expect("Failed to lock guest ID mutex");
        let guest_id = *id_lock;
        *id_lock += 1;
        guest_id
    }

    fn more_rights(this: &Player, target: &Player) -> bool {
        (this.is_admin && !target.is_admin) || (this.is_mod && !target.is_admin && !target.is_mod)
    }

    fn more_rights_and_admin(this: &Player, target: &Player) -> bool {
        this.is_admin && !target.is_admin
    }

    fn update_player(
        &self,
        username: &PlayerUsername,
        target_username: &PlayerUsername,
        access_predicate: impl Fn(&Player, &Player) -> bool,
        database_update_fn: impl Fn(
            &Player,
            &PooledConnection<SqliteConnectionManager>,
        ) -> Result<usize, rusqlite::Error>,
    ) -> ServiceResult<()> {
        let Some(current_player) = self.fetch_player(&username) else {
            return ServiceError::not_found("Current player not found");
        };
        let Some(player) = self.fetch_player(target_username) else {
            return ServiceError::not_found("Target player not found");
        };
        if !access_predicate(&current_player, &player) {
            return ServiceError::unauthorized("Insufficient rights");
        }
        let conn = PLAYER_DB_POOL
            .get()
            .map_err(|e| DatabaseError::ConnectionError(e))?;
        database_update_fn(&player, &conn).map_err(|e| DatabaseError::QueryError(e))?;
        self.player_cache.invalidate(target_username);
        Ok(())
    }

    fn uniquify_username(username: &PlayerUsername) -> PlayerUsername {
        username
            .to_ascii_lowercase()
            .replace("_", "")
            .replace("i", "1")
            .replace("l", "1")
            .replace("o", "0")
    }

    fn try_take_username(&self, username: &PlayerUsername) -> ServiceResult<()> {
        let unique_username = Self::uniquify_username(username);
        if self.taken_unique_usernames.contains_key(&unique_username) {
            return ServiceError::not_possible("Username already taken");
        }
        self.taken_unique_usernames.insert(unique_username, ());
        Ok(())
    }

    fn create_player(&self, username: &PlayerUsername, email: &str) -> ServiceResult<()> {
        let temp_password = Self::generate_temporary_password();
        let password_hash = bcrypt::hash(&temp_password, bcrypt::DEFAULT_COST).unwrap();
        let conn = PLAYER_DB_POOL
            .get()
            .map_err(|e| DatabaseError::ConnectionError(e))?;
        let largest_player_id: i32 = conn
            .query_row("SELECT MAX(id) FROM players", [], |row| {
                row.get::<_, i32>(0)
            })
            .map_err(|e| DatabaseError::QueryError(e))?;
        conn.execute(
            "INSERT INTO players (id, name, password, email) VALUES (?1, ?2, ?3, ?4)",
            (largest_player_id + 1, username, password_hash, email),
        )
        .map_err(|e| DatabaseError::QueryError(e))?;
        self.send_password_email(email, username, &temp_password)?;
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
        &self,
        to: &str,
        username: &PlayerUsername,
        temp_password: &str,
    ) -> ServiceResult<()> {
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
        self.email_service.send_email(to, &subject, &body)?;
        Ok(())
    }

    fn send_reset_token_email(
        &self,
        to: &str,
        username: &PlayerUsername,
        reset_token: &str,
    ) -> ServiceResult<()> {
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
        self.email_service.send_email(to, &subject, &body)
    }

    fn send_ban_email(
        &self,
        to: &str,
        username: &PlayerUsername,
        ban_msg: &str,
    ) -> ServiceResult<()> {
        let subject = "Playtak Account Banned";
        let body = format!(
            "Hello {},\n\n\
        Your account has been banned for the following reason:\n\
        {}\n\n\
        If you believe this is a mistake, please contact support.\n\n\
        Best regards,\n\
        The Playtak Team",
            username, ban_msg
        );
        self.email_service.send_email(to, &subject, &body)
    }

    fn update_password(&self, username: &PlayerUsername, new_password: &str) -> ServiceResult<()> {
        let Some(player) = self.fetch_player(&username) else {
            return ServiceError::not_found("Player not found");
        };
        let password_hash = bcrypt::hash(new_password, bcrypt::DEFAULT_COST)
            .map_err(|e| ServiceError::Internal(format!("Failed to hash password: {}", e)))?;

        let conn = PLAYER_DB_POOL
            .get()
            .map_err(|e| DatabaseError::ConnectionError(e))?;

        conn.execute(
            "UPDATE players SET password = ?1 WHERE id = ?2",
            (password_hash, player.id),
        )
        .map_err(|e| DatabaseError::QueryError(e))?;
        self.player_cache.invalidate(username);
        Ok(())
    }
}

impl PlayerService for PlayerServiceImpl {
    fn load_unique_usernames(&self) -> ServiceResult<()> {
        let conn = PLAYER_DB_POOL
            .get()
            .map_err(|e| DatabaseError::ConnectionError(e))?;
        let mut stmt = conn
            .prepare("SELECT name FROM players")
            .map_err(|e| DatabaseError::QueryError(e))?;
        let usernames = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| DatabaseError::QueryError(e))?;
        for username in usernames {
            let username: String = username.map_err(|e| DatabaseError::QueryError(e))?;
            let unique_username = Self::uniquify_username(&username);
            self.taken_unique_usernames.insert(unique_username, ());
        }
        Ok(())
    }

    fn fetch_player(&self, username: &str) -> Option<Player> {
        if username.starts_with("Guest") {
            return None;
        }
        let username = username.to_string();
        if let Some(player) = self.player_cache.get(&username) {
            return Some(player);
        }
        let player = PLAYER_DB_POOL.get().ok()?.query_one(
            "SELECT * FROM players WHERE name = ?1",
            [username.clone()],
            |row| {
                Ok(Player {
                    password_hash: row.get("password")?,
                    username: row.get("name")?,
                    rating: row.get("rating")?,
                    id: row.get("id")?,
                    email: row.get("email")?,
                    is_bot: row.get::<_, i32>("isbot")? != 0,
                    is_gagged: row.get::<_, i32>("is_gagged")? != 0,
                    is_mod: row.get::<_, i32>("is_mod")? != 0,
                    is_admin: row.get::<_, i32>("is_admin")? != 0,
                })
            },
        );
        match player {
            Ok(p) => {
                self.player_cache.insert(username.clone(), p.clone());
                Some(p)
            }
            Err(_) => None,
        }
    }

    fn set_gagged(
        &self,
        username: &PlayerUsername,
        target_username: &PlayerUsername,
        gagged: bool,
    ) -> ServiceResult<()> {
        self.update_player(
            username,
            target_username,
            Self::more_rights,
            |player, conn| {
                conn.execute(
                    "UPDATE players SET is_gagged = ?1 WHERE id = ?2",
                    (gagged as i32, player.id),
                )
            },
        )
    }

    fn set_banned(
        &self,
        id: &ClientId,
        username: &PlayerUsername,
        target_username: &PlayerUsername,
        banned: Option<String>,
    ) -> ServiceResult<()> {
        self.update_player(
            username,
            target_username,
            Self::more_rights,
            |player, conn| {
                conn.execute(
                    "UPDATE players SET is_banned = ?1 WHERE id = ?2",
                    (banned.is_some() as i32, player.id),
                )
            },
        )?;
        if let Some(ban_msg) = banned {
            self.client_service.close_client(id);
            if let Some(target_player) = self.fetch_player(target_username) {
                self.send_ban_email(&target_player.email, target_username, &ban_msg)?;
            } else {
                return ServiceError::not_found("Target player not found");
            }
        }
        Ok(())
    }

    fn set_modded(
        &self,
        username: &PlayerUsername,
        target_username: &PlayerUsername,
        modded: bool,
    ) -> ServiceResult<()> {
        self.update_player(
            username,
            target_username,
            Self::more_rights_and_admin,
            |player, conn| {
                conn.execute(
                    "UPDATE players SET is_mod = ?1 WHERE id = ?2",
                    (modded as i32, player.id),
                )
            },
        )
    }

    fn set_admin(
        &self,
        username: &PlayerUsername,
        target_username: &PlayerUsername,
        admin: bool,
    ) -> ServiceResult<()> {
        self.update_player(
            username,
            target_username,
            Self::more_rights_and_admin,
            |player, conn| {
                conn.execute(
                    "UPDATE players SET is_admin = ?1 WHERE id = ?2",
                    (admin as i32, player.id),
                )
            },
        )
    }

    fn set_bot(
        &self,
        username: &PlayerUsername,
        target_username: &PlayerUsername,
        bot: bool,
    ) -> ServiceResult<()> {
        self.update_player(
            username,
            target_username,
            Self::more_rights_and_admin,
            |player, conn| {
                conn.execute(
                    "UPDATE players SET isbot = ?1 WHERE id = ?2",
                    (bot as i32, player.id),
                )
            },
        )
    }

    fn validate_login(&self, username: &PlayerUsername, password: &str) -> ServiceResult<()> {
        if let Some(player) = self.fetch_player(&username) {
            bcrypt::verify(password, &player.password_hash)
                .map_err(|_| ServiceError::Unauthorized("Invalid username or password".into()))
                .and_then(|is_valid| {
                    if is_valid {
                        Ok(())
                    } else {
                        Err(ServiceError::Unauthorized(
                            "Invalid username or password".into(),
                        ))
                    }
                })
        } else {
            Err(ServiceError::Unauthorized(
                "Invalid username or password".into(),
            ))
        }
    }

    fn try_login(
        &self,
        id: &ClientId,
        username: &PlayerUsername,
        password: &str,
    ) -> ServiceResult<()> {
        self.validate_login(username, password)?;
        self.client_service.associate_player(id, username)
    }

    fn try_login_jwt(&self, id: &ClientId, token: &str) -> ServiceResult<PlayerUsername> {
        let username =
            validate_jwt(token).ok_or(ServiceError::Unauthorized("Invalid token".into()))?;
        self.client_service.associate_player(id, &username)?;
        Ok(username)
    }

    fn try_login_guest(&self, id: &ClientId, token: Option<&str>) -> ServiceResult<PlayerUsername> {
        let guest_name = token
            .and_then(|x| self.guest_player_tokens.get(x))
            .unwrap_or_else(|| format!("Guest{}", self.increment_guest_id()));

        self.client_service.associate_player(id, &guest_name)?;
        if let Some(token) = token {
            self.guest_player_tokens
                .insert(guest_name.clone(), token.to_string());
        }
        Ok(guest_name)
    }

    fn try_register(&self, username: &PlayerUsername, email: &str) -> ServiceResult<()> {
        if username.to_ascii_lowercase().starts_with("guest") {
            return ServiceError::validation_err("Username cannot start with 'Guest'");
        }
        if username.is_inappropriate() {
            return ServiceError::validation_err("Username contains inappropriate content");
        }
        if username.len() < 3 || username.len() > 15 {
            return ServiceError::validation_err("Username must be between 3 and 15 characters");
        }
        if username
            .chars()
            .next()
            .is_none_or(|c| !c.is_ascii_alphabetic())
        {
            return ServiceError::validation_err("Username must start with a letter");
        }
        if username
            .chars()
            .any(|c| !c.is_ascii_alphanumeric() && c != '_')
        {
            return ServiceError::validation_err("Username must be alphanumeric");
        }
        let email_validator = EmailValidator {
            email: email.to_string(),
        };
        if let Err(e) = email_validator.validate() {
            return ServiceError::validation_err(format!("Invalid email: {}", e));
        }
        self.try_take_username(username)?;
        self.create_player(username, email)
    }

    fn send_reset_token(&self, username: &PlayerUsername, email: &str) -> ServiceResult<()> {
        let Some(player) = self.fetch_player(username) else {
            return ServiceError::not_found("Player not found");
        };
        if player.email != email {
            return ServiceError::validation_err("Email does not match");
        }
        let reset_token = Self::generate_temporary_password();
        self.password_reset_tokens
            .insert(reset_token.clone(), (username.clone(), Instant::now()));
        self.send_reset_token_email(email, username, &reset_token)?;
        Ok(())
    }

    fn reset_password(
        &self,
        username: &PlayerUsername,
        reset_token: &str,
        new_password: &str,
    ) -> ServiceResult<()> {
        let Some(player) = self.fetch_player(username) else {
            return ServiceError::not_found("Player not found");
        };

        let Some((token_username, token_time)) = self.password_reset_tokens.remove(reset_token)
        else {
            return ServiceError::validation_err("Invalid or expired reset token for this user");
        };
        if &token_username != username {
            return ServiceError::validation_err("Invalid or expired reset token for this user");
        }
        if token_time.elapsed() > PASSWORD_RESET_TOKEN_TTL {
            return ServiceError::validation_err("Invalid or expired reset token for this user");
        }

        let password_hash = bcrypt::hash(new_password, bcrypt::DEFAULT_COST)
            .map_err(|e| ServiceError::Internal(format!("Failed to hash password: {}", e)))?;

        let conn = PLAYER_DB_POOL
            .get()
            .map_err(|e| DatabaseError::ConnectionError(e))?;

        conn.execute(
            "UPDATE players SET password = ?1 WHERE id = ?2",
            (password_hash, player.id),
        )
        .map_err(|e| DatabaseError::QueryError(e))?;
        self.player_cache.invalidate(username);
        Ok(())
    }

    fn change_password(
        &self,
        username: &PlayerUsername,
        current_password: &str,
        new_password: &str,
    ) -> ServiceResult<()> {
        self.validate_login(&username, current_password)?;
        self.update_password(username, new_password)
    }

    fn set_password(
        &self,
        username: &PlayerUsername,
        target_username: &PlayerUsername,
        new_password: &str,
    ) -> ServiceResult<()> {
        let Some(player) = self.fetch_player(&username) else {
            return ServiceError::not_found("Player not found");
        };
        if !player.is_admin {
            return ServiceError::unauthorized("Only admins can set passwords directly");
        }
        self.update_password(target_username, new_password)
    }

    fn get_players(
        &self,
        ban_filter: Option<bool>,
        gag_filter: Option<bool>,
        mod_filter: Option<bool>,
        admin_filter: Option<bool>,
        bot_filter: Option<bool>,
    ) -> ServiceResult<Vec<Player>> {
        let conn = PLAYER_DB_POOL
            .get()
            .map_err(|e| DatabaseError::ConnectionError(e))?;
        let mut query = "SELECT * FROM players".to_string();
        let mut param_index = 1;
        let filters = vec![
            ("is_banned", ban_filter),
            ("is_gagged", gag_filter),
            ("is_mod", mod_filter),
            ("is_admin", admin_filter),
            ("isbot", bot_filter),
        ]
        .into_iter()
        .filter_map(|(condition, f)| {
            if let Some(v) = f {
                Some((condition, v))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
        let params = rusqlite::params_from_iter(filters.iter().map(|(_, v)| *v as i32));
        if !filters.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(
                &filters
                    .iter()
                    .map(|(filter, _)| {
                        let cond = format!("{} = ?{}", filter, param_index);
                        param_index += 1;
                        cond
                    })
                    .collect::<Vec<_>>()
                    .join(" AND "),
            );
        }
        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| DatabaseError::QueryError(e))?;
        let players = stmt.query_map(params, |row| {
            Ok(Player {
                password_hash: row.get("password")?,
                username: row.get("name")?,
                rating: row.get("rating")?,
                id: row.get("id")?,
                email: row.get("email")?,
                is_bot: row.get::<_, i32>("isbot")? != 0,
                is_gagged: row.get::<_, i32>("is_gagged")? != 0,
                is_mod: row.get::<_, i32>("is_mod")? != 0,
                is_admin: row.get::<_, i32>("is_admin")? != 0,
            })
        });
        let result = players
            .map_err(|e| DatabaseError::QueryError(e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| DatabaseError::QueryError(e))?;
        Ok(result)
    }
}
