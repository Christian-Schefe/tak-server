use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use dashmap::DashMap;
use passwords::PasswordGenerator;
use rustrict::CensorStr;

use crate::{
    ServiceError, ServiceResult,
    email::ArcEmailService,
    jwt::ArcJwtService,
    transport::{ArcTransportService, DisconnectReason, ServerMessage},
    util::validate_email,
};

pub type PlayerUsername = String;

const GUEST_TTL: Duration = Duration::from_secs(60 * 60 * 4);

const PASSWORD_RESET_TOKEN_TTL: Duration = Duration::from_secs(60 * 60 * 24);

#[derive(Clone)]
pub struct Player {
    pub username: PlayerUsername,
    pub email: Option<String>,
    pub rating: f64,
    pub password_hash: Option<String>,
    pub is_bot: bool,
    pub is_gagged: bool,
    pub is_mod: bool,
    pub is_admin: bool,
    pub is_banned: bool,
}

#[derive(Clone, Default)]
pub struct PlayerUpdate {
    pub password_hash: Option<String>,
    pub is_bot: Option<bool>,
    pub is_gagged: Option<bool>,
    pub is_mod: Option<bool>,
    pub is_admin: Option<bool>,
    pub is_banned: Option<bool>,
}

pub struct PlayerFilter {
    pub is_bot: Option<bool>,
    pub is_gagged: Option<bool>,
    pub is_mod: Option<bool>,
    pub is_admin: Option<bool>,
    pub is_banned: Option<bool>,
}

pub type ArcPlayerRepository = Arc<Box<dyn PlayerRepository + Send + Sync + 'static>>;
pub trait PlayerRepository {
    fn get_player_by_id(&self, id: i64) -> ServiceResult<Option<Player>>;
    fn get_player_by_name(&self, name: &str) -> ServiceResult<Option<(PlayerId, Player)>>;
    fn create_player(&self, player: &Player) -> ServiceResult<()>;
    fn update_player(&self, id: i64, update: &PlayerUpdate) -> ServiceResult<()>;
    fn get_players(&self, filter: PlayerFilter) -> ServiceResult<Vec<Player>>;
    fn get_player_names(&self) -> ServiceResult<Vec<String>>;
}

pub type ArcPlayerService = Arc<Box<dyn PlayerService + Send + Sync + 'static>>;
pub trait PlayerService {
    fn load_unique_usernames(&self) -> ServiceResult<()>;
    fn fetch_player(&self, username: &str) -> ServiceResult<(Option<PlayerId>, Player)>;
    fn fetch_player_data(&self, username: &str) -> ServiceResult<Player> {
        let (_, player) = self.fetch_player(username)?;
        Ok(player)
    }
    fn validate_login(&self, username: &PlayerUsername, password: &str) -> ServiceResult<()>;
    fn try_login(&self, username: &PlayerUsername, password: &str)
    -> ServiceResult<PlayerUsername>;
    fn try_login_jwt(&self, token: &str) -> ServiceResult<PlayerUsername>;
    fn try_login_guest(&self, token: Option<&str>) -> ServiceResult<PlayerUsername>;
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
    fn try_kick(
        &self,
        username: &PlayerUsername,
        target_username: &PlayerUsername,
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

type PlayerId = i64;

pub struct PlayerServiceImpl {
    transport_service: ArcTransportService,
    email_service: ArcEmailService,
    jwt_service: ArcJwtService,
    player_repository: ArcPlayerRepository,
    player_cache: Arc<moka::sync::Cache<PlayerUsername, (Option<PlayerId>, Player)>>,
    guests: Arc<DashMap<PlayerUsername, Player>>,
    guest_player_tokens: Arc<moka::sync::Cache<String, PlayerUsername>>,
    next_guest_id: Arc<std::sync::Mutex<u32>>,
    taken_unique_usernames: Arc<DashMap<PlayerUsername, ()>>,
    password_reset_tokens: Arc<moka::sync::Cache<String, (PlayerUsername, Instant)>>,
}

impl PlayerServiceImpl {
    pub fn new(
        transport_service: ArcTransportService,
        email_service: ArcEmailService,
        jwt_service: ArcJwtService,
        player_repository: ArcPlayerRepository,
    ) -> Self {
        Self {
            transport_service,
            email_service,
            jwt_service,
            player_repository,
            player_cache: Arc::new(moka::sync::Cache::builder().max_capacity(1000).build()),
            guests: Arc::new(DashMap::new()),
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
        let (id, _) = self.fetch_player(&username)?;
        let Some(id) = id else {
            return ServiceError::not_possible("Player is a guest");
        };
        let password_hash = bcrypt::hash(new_password, bcrypt::DEFAULT_COST)
            .map_err(|e| ServiceError::Internal(format!("Failed to hash password: {}", e)))?;

        let update = PlayerUpdate {
            password_hash: Some(password_hash.clone()),
            ..Default::default()
        };

        self.player_repository.update_player(id, &update)?;
        self.player_cache.invalidate(username);
        Ok(())
    }

    fn update_player(
        &self,
        username: &PlayerUsername,
        target_username: &PlayerUsername,
        access_predicate: impl Fn(&Player, &Player) -> bool,
        update: &PlayerUpdate,
    ) -> ServiceResult<()> {
        let current_player = self.fetch_player_data(&username)?;
        let (id, player) = self.fetch_player(target_username)?;
        if let Some(id) = id {
            if !access_predicate(&current_player, &player) {
                return ServiceError::unauthorized("Insufficient rights");
            }
            self.player_repository.update_player(id, update)?;
            self.player_cache.invalidate(target_username);
            Ok(())
        } else {
            let Some(mut player) = self.guests.get_mut(target_username) else {
                return ServiceError::not_found("Player not found");
            };
            if !access_predicate(&current_player, &player) {
                return ServiceError::unauthorized("Insufficient rights");
            }
            if let Some(is_bot) = update.is_bot {
                player.is_bot = is_bot;
            }
            if let Some(is_gagged) = update.is_gagged {
                player.is_gagged = is_gagged;
            }
            if let Some(is_mod) = update.is_mod {
                player.is_mod = is_mod;
            }
            if let Some(is_admin) = update.is_admin {
                player.is_admin = is_admin;
            }
            if let Some(is_banned) = update.is_banned {
                player.is_banned = is_banned;
            }
            Ok(())
        }
    }

    fn validate_username(username: &PlayerUsername) -> ServiceResult<()> {
        if username.to_ascii_lowercase().starts_with("guest") {
            return ServiceError::bad_request("Username cannot start with 'Guest'");
        }
        if username.is_inappropriate() {
            return ServiceError::bad_request("Username contains inappropriate content");
        }
        if username.len() < 3 || username.len() > 15 {
            return ServiceError::bad_request("Username must be between 3 and 15 characters");
        }
        if username
            .chars()
            .next()
            .is_none_or(|c| !c.is_ascii_alphabetic())
        {
            return ServiceError::bad_request("Username must start with a letter");
        }
        if username
            .chars()
            .any(|c| !c.is_ascii_alphanumeric() && c != '_')
        {
            return ServiceError::bad_request("Username must be alphanumeric");
        }
        Ok(())
    }
}

impl PlayerService for PlayerServiceImpl {
    fn load_unique_usernames(&self) -> ServiceResult<()> {
        let usernames = self.player_repository.get_player_names()?;
        for username in usernames {
            let unique_username = Self::uniquify_username(&username);
            self.taken_unique_usernames.insert(unique_username, ());
        }
        Ok(())
    }

    fn fetch_player(&self, username: &str) -> ServiceResult<(Option<PlayerId>, Player)> {
        if username.starts_with("Guest") {
            let Some(guest) = self.guests.get(username).map(|entry| entry.value().clone()) else {
                return ServiceError::not_found("Player not found");
            };
            return Ok((None, guest));
        }
        let username = username.to_string();
        if let Some(player) = self.player_cache.get(&username) {
            return Ok(player);
        }
        let player = self.player_repository.get_player_by_name(&username)?;
        match player {
            Some((id, p)) => {
                let val = (Some(id), p);
                self.player_cache.insert(username.clone(), val.clone());
                Ok(val)
            }
            None => ServiceError::not_found("Player not found"),
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
            &PlayerUpdate {
                is_gagged: Some(gagged),
                ..Default::default()
            },
        )?;
        println!(
            "User {} set gagged={} for user {}",
            username, gagged, target_username
        );
        Ok(())
    }

    fn set_banned(
        &self,
        username: &PlayerUsername,
        target_username: &PlayerUsername,
        banned: Option<String>,
    ) -> ServiceResult<()> {
        self.update_player(
            username,
            target_username,
            Self::more_rights,
            &PlayerUpdate {
                is_banned: Some(banned.is_some()),
                ..Default::default()
            },
        )?;
        if let Some(ban_msg) = &banned {
            self.transport_service.try_player_send(
                &target_username,
                &ServerMessage::ConnectionClosed {
                    reason: DisconnectReason::Ban(ban_msg.clone()),
                },
            );

            let target_player = self.fetch_player_data(target_username)?;
            if let Some(player_email) = &target_player.email
                && let Ok(email) = validate_email(player_email)
            {
                self.send_ban_email(&email, target_username, ban_msg)?;
            }
        }
        println!(
            "User {} set banned={} for user {}: {}",
            banned.is_some(),
            username,
            target_username,
            banned.unwrap_or("No reason provided".into())
        );
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
            &PlayerUpdate {
                is_mod: Some(modded),
                ..Default::default()
            },
        )?;
        println!(
            "User {} set modded={} for user {}",
            username, modded, target_username
        );
        Ok(())
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
            &PlayerUpdate {
                is_admin: Some(admin),
                ..Default::default()
            },
        )?;
        println!(
            "User {} set admin={} for user {}",
            username, admin, target_username
        );
        Ok(())
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
            &PlayerUpdate {
                is_bot: Some(bot),
                ..Default::default()
            },
        )?;
        println!(
            "User {} set bot={} for user {}",
            username, bot, target_username
        );
        Ok(())
    }

    fn try_kick(
        &self,
        username: &PlayerUsername,
        target_username: &PlayerUsername,
    ) -> ServiceResult<()> {
        let current_player = self.fetch_player_data(&username)?;
        let target_player = self.fetch_player_data(&target_username)?;
        if !Self::more_rights(&current_player, &target_player) {
            return ServiceError::unauthorized("Insufficient rights to kick this player");
        }

        self.transport_service.try_player_send(
            &target_username,
            &ServerMessage::ConnectionClosed {
                reason: DisconnectReason::Kick,
            },
        );
        println!("User {} kicked user {}", username, target_username);

        Ok(())
    }

    fn validate_login(&self, username: &PlayerUsername, password: &str) -> ServiceResult<()> {
        let player = self.fetch_player_data(&username)?;
        let Some(password_hash) = &player.password_hash else {
            return ServiceError::unauthorized("Invalid username or password");
        };
        let valid = bcrypt::verify(password, password_hash)
            .map_err(|_| ServiceError::BadRequest("Failed to hash password".into()))?;
        println!(
            "Login attempt for user {}: {}, {}",
            username,
            password,
            if valid { "success" } else { "failure" }
        );
        if !valid {
            return Err(ServiceError::Unauthorized(
                "Invalid username or password".into(),
            ));
        }
        Ok(())
    }

    fn try_login(
        &self,
        username: &PlayerUsername,
        password: &str,
    ) -> ServiceResult<PlayerUsername> {
        self.validate_login(username, password)?;
        let player = self.fetch_player_data(username)?;
        if player.is_banned {
            return ServiceError::unauthorized("User is banned");
        }
        Ok(username.clone())
    }

    fn try_login_jwt(&self, token: &str) -> ServiceResult<PlayerUsername> {
        let username = self.jwt_service.validate_jwt(token)?;
        let player = self.fetch_player_data(&username)?;
        if player.is_banned {
            return ServiceError::unauthorized("User is banned");
        }
        Ok(username)
    }

    fn try_login_guest(&self, token: Option<&str>) -> ServiceResult<PlayerUsername> {
        let valid_token = token.map(|t| self.guest_player_tokens.contains_key(t));
        let guest_name = token
            .and_then(|x| self.guest_player_tokens.get(x))
            .unwrap_or_else(|| format!("Guest{}", self.increment_guest_id()));

        if let Some(token) = token {
            self.guest_player_tokens
                .insert(guest_name.clone(), token.to_string());
        }
        //reset guest player if no or new token
        if !matches!(valid_token, Some(true)) {
            self.guests.remove(&guest_name);
        }
        self.guests
            .entry(guest_name.clone())
            .or_insert_with(|| Player {
                username: guest_name.clone(),
                email: None,
                rating: 1000.0,
                password_hash: None,
                is_bot: false,
                is_gagged: false,
                is_mod: false,
                is_admin: false,
                is_banned: false,
            });
        Ok(guest_name)
    }

    fn try_register(&self, username: &PlayerUsername, email: &str) -> ServiceResult<()> {
        Self::validate_username(username)?;

        let email = validate_email(email)?;
        self.try_take_username(username)?;
        let temp_password = Self::generate_temporary_password();
        let password_hash = bcrypt::hash(&temp_password, bcrypt::DEFAULT_COST).unwrap();
        self.player_repository.create_player(&Player {
            username: username.clone(),
            email: Some(email.to_string()),
            rating: 1000.0,
            password_hash: Some(password_hash),
            is_bot: false,
            is_gagged: false,
            is_mod: false,
            is_admin: false,
            is_banned: false,
        })?;
        self.send_password_email(&email, username, &temp_password)?;
        Ok(())
    }

    fn send_reset_token(&self, username: &PlayerUsername, email: &str) -> ServiceResult<()> {
        let player = self.fetch_player_data(username)?;
        if player.email.is_none_or(|e| e != email) {
            return ServiceError::bad_request("Email does not match");
        }
        let email = validate_email(email)?;
        let reset_token = Self::generate_temporary_password();
        self.password_reset_tokens
            .insert(reset_token.clone(), (username.clone(), Instant::now()));
        self.send_reset_token_email(&email, username, &reset_token)?;
        Ok(())
    }

    fn reset_password(
        &self,
        username: &PlayerUsername,
        reset_token: &str,
        new_password: &str,
    ) -> ServiceResult<()> {
        let Some((token_username, token_time)) = self.password_reset_tokens.remove(reset_token)
        else {
            return ServiceError::bad_request("Invalid or expired reset token for this user");
        };
        if &token_username != username {
            return ServiceError::bad_request("Invalid or expired reset token for this user");
        }
        if token_time.elapsed() > PASSWORD_RESET_TOKEN_TTL {
            return ServiceError::bad_request("Invalid or expired reset token for this user");
        }

        self.update_password(username, new_password)
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
        let player = self.fetch_player_data(&username)?;
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
        let players = self.player_repository.get_players(PlayerFilter {
            is_banned: ban_filter,
            is_gagged: gag_filter,
            is_mod: mod_filter,
            is_admin: admin_filter,
            is_bot: bot_filter,
        })?;
        Ok(players)
    }
}

#[derive(Default, Clone)]
pub struct MockPlayerService;

impl PlayerService for MockPlayerService {
    fn load_unique_usernames(&self) -> ServiceResult<()> {
        Ok(())
    }

    fn fetch_player(&self, username: &str) -> ServiceResult<(Option<PlayerId>, Player)> {
        match username {
            "test_admin" => Ok((
                Some(1),
                Player {
                    username: "test_admin".into(),
                    email: Some("test_admin@example.com".into()),
                    rating: 1500.0,
                    password_hash: Some("".to_string()),
                    is_bot: false,
                    is_gagged: false,
                    is_mod: true,
                    is_admin: true,
                    is_banned: false,
                },
            )),
            "test_gagged" => Ok((
                Some(2),
                Player {
                    username: "test_gagged".into(),
                    email: Some("test_gagged@example.com".into()),
                    rating: 1200.0,
                    password_hash: Some("".to_string()),
                    is_bot: false,
                    is_gagged: true,
                    is_mod: false,
                    is_admin: false,
                    is_banned: false,
                },
            )),
            _ => ServiceError::not_found("Player not found"),
        }
    }

    fn validate_login(&self, _username: &PlayerUsername, _password: &str) -> ServiceResult<()> {
        Ok(())
    }

    fn try_login(
        &self,
        _username: &PlayerUsername,
        _password: &str,
    ) -> ServiceResult<PlayerUsername> {
        Ok("".to_string())
    }

    fn try_login_jwt(&self, _token: &str) -> ServiceResult<PlayerUsername> {
        Ok("".to_string())
    }

    fn try_login_guest(&self, _token: Option<&str>) -> ServiceResult<PlayerUsername> {
        Ok("".to_string())
    }

    fn try_register(&self, _username: &PlayerUsername, _email: &str) -> ServiceResult<()> {
        Ok(())
    }

    fn send_reset_token(&self, _username: &PlayerUsername, _email: &str) -> ServiceResult<()> {
        Ok(())
    }

    fn reset_password(
        &self,
        _username: &PlayerUsername,
        _reset_token: &str,
        _new_password: &str,
    ) -> ServiceResult<()> {
        Ok(())
    }

    fn change_password(
        &self,
        _username: &PlayerUsername,
        _current_password: &str,
        _new_password: &str,
    ) -> ServiceResult<()> {
        Ok(())
    }

    fn set_gagged(
        &self,
        _username: &PlayerUsername,
        _target_username: &PlayerUsername,
        _gagged: bool,
    ) -> ServiceResult<()> {
        Ok(())
    }

    fn set_banned(
        &self,
        _username: &PlayerUsername,
        _target_username: &PlayerUsername,
        _banned: Option<String>,
    ) -> ServiceResult<()> {
        Ok(())
    }

    fn set_modded(
        &self,
        _username: &PlayerUsername,
        _target_username: &PlayerUsername,
        _modded: bool,
    ) -> ServiceResult<()> {
        Ok(())
    }

    fn set_admin(
        &self,
        _username: &PlayerUsername,
        _target_username: &PlayerUsername,
        _admin: bool,
    ) -> ServiceResult<()> {
        Ok(())
    }

    fn set_bot(
        &self,
        _username: &PlayerUsername,
        _target_username: &PlayerUsername,
        _bot: bool,
    ) -> ServiceResult<()> {
        Ok(())
    }

    fn try_kick(
        &self,
        _username: &PlayerUsername,
        _target_username: &PlayerUsername,
    ) -> ServiceResult<()> {
        Ok(())
    }

    fn get_players(
        &self,
        _ban_filter: Option<bool>,
        _gag_filter: Option<bool>,
        _mod_filter: Option<bool>,
        _admin_filter: Option<bool>,
        _bot_filter: Option<bool>,
    ) -> ServiceResult<Vec<Player>> {
        Ok(vec![])
    }

    fn set_password(
        &self,
        _username: &PlayerUsername,
        _target_username: &PlayerUsername,
        _new_password: &str,
    ) -> ServiceResult<()> {
        Ok(())
    }
}
