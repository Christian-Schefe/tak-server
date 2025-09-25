use dashmap::DashMap;
use passwords::PasswordGenerator;
use rustrict::CensorStr;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    ArcClientService, ArcEmailService, ArcPlayerRepository, ServiceError, ServiceResult,
    client::ClientId,
    jwt::validate_jwt,
    persistence::players::{PlayerFilter, PlayerUpdate},
    util::validate_email,
};

pub type PlayerUsername = String;

const GUEST_TTL: Duration = Duration::from_secs(60 * 60 * 4);

const PASSWORD_RESET_TOKEN_TTL: Duration = Duration::from_secs(60 * 60 * 24);

#[derive(Clone)]
pub struct Player {
    pub id: i64,
    pub username: PlayerUsername,
    pub email: String,
    pub rating: f64,
    pub password_hash: String,
    pub is_bot: bool,
    pub is_gagged: bool,
    pub is_mod: bool,
    pub is_admin: bool,
    pub is_banned: bool,
}

pub trait PlayerService {
    fn load_unique_usernames(&self) -> ServiceResult<()>;
    fn fetch_player(&self, username: &str) -> ServiceResult<Player>;
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

pub struct PlayerServiceImpl {
    client_service: ArcClientService,
    email_service: ArcEmailService,
    player_repository: ArcPlayerRepository,
    player_cache: Arc<moka::sync::Cache<PlayerUsername, Player>>,
    guest_player_tokens: Arc<moka::sync::Cache<String, PlayerUsername>>,
    next_guest_id: Arc<std::sync::Mutex<u32>>,
    taken_unique_usernames: Arc<DashMap<PlayerUsername, ()>>,
    password_reset_tokens: Arc<moka::sync::Cache<String, (PlayerUsername, Instant)>>,
}

impl PlayerServiceImpl {
    pub fn new(
        client_service: ArcClientService,
        email_service: ArcEmailService,
        player_repository: ArcPlayerRepository,
    ) -> Self {
        Self {
            client_service,
            email_service,
            player_repository,
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
        let player = self.fetch_player(&username)?;
        let password_hash = bcrypt::hash(new_password, bcrypt::DEFAULT_COST)
            .map_err(|e| ServiceError::Internal(format!("Failed to hash password: {}", e)))?;

        let update = PlayerUpdate {
            password_hash: Some(password_hash.clone()),
            ..Default::default()
        };

        self.player_repository.update_player(player.id, &update)?;
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
        let current_player = self.fetch_player(&username)?;
        let player = self.fetch_player(target_username)?;
        if !access_predicate(&current_player, &player) {
            return ServiceError::unauthorized("Insufficient rights");
        }
        self.player_repository.update_player(player.id, update)?;
        self.player_cache.invalidate(target_username);
        Ok(())
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

    fn fetch_player(&self, username: &str) -> ServiceResult<Player> {
        if username.starts_with("Guest") {
            return ServiceError::not_found("Player not found");
        }
        let username = username.to_string();
        if let Some(player) = self.player_cache.get(&username) {
            return Ok(player);
        }
        let player = self.player_repository.get_player_by_name(&username)?;
        match player {
            Some(p) => {
                self.player_cache.insert(username.clone(), p.clone());
                Ok(p)
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
            if let Some(target_id) = self.client_service.get_associated_client(target_username) {
                self.client_service.close_client(&target_id);
            }
            let target_player = self.fetch_player(target_username)?;
            if let Ok(email) = validate_email(&target_player.email) {
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
        let current_player = self.fetch_player(&username)?;
        let target_player = self.fetch_player(&target_username)?;
        if !Self::more_rights(&current_player, &target_player) {
            return ServiceError::unauthorized("Insufficient rights to kick this player");
        }
        if let Some(target_id) = self.client_service.get_associated_client(target_username) {
            self.client_service.close_client(&target_id);
            println!("User {} kicked user {}", username, target_username);
        }
        Ok(())
    }

    fn validate_login(&self, username: &PlayerUsername, password: &str) -> ServiceResult<()> {
        let player = self.fetch_player(&username)?;

        let valid = bcrypt::verify(password, &player.password_hash)
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
        id: &ClientId,
        username: &PlayerUsername,
        password: &str,
    ) -> ServiceResult<()> {
        self.validate_login(username, password)?;
        let player = self.fetch_player(username)?;
        if player.is_banned {
            return ServiceError::unauthorized("User is banned");
        }
        self.client_service.associate_player(id, username)
    }

    fn try_login_jwt(&self, id: &ClientId, token: &str) -> ServiceResult<PlayerUsername> {
        let username =
            validate_jwt(token).ok_or(ServiceError::Unauthorized("Invalid token".into()))?;
        let player = self.fetch_player(&username)?;
        if player.is_banned {
            return ServiceError::unauthorized("User is banned");
        }
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
        Self::validate_username(username)?;

        let email = validate_email(email)?;
        self.try_take_username(username)?;
        let temp_password = Self::generate_temporary_password();
        let password_hash = bcrypt::hash(&temp_password, bcrypt::DEFAULT_COST).unwrap();
        self.player_repository.create_player(&Player {
            id: 0, // Will be set by the database
            username: username.clone(),
            email: email.to_string(),
            rating: 1000.0,
            password_hash,
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
        let player = self.fetch_player(username)?;
        if player.email != email {
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
        let player = self.fetch_player(&username)?;
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

    fn fetch_player(&self, username: &str) -> ServiceResult<Player> {
        match username {
            "test_admin" => Ok(Player {
                id: 1,
                username: "test_admin".into(),
                email: "test_admin@example.com".into(),
                rating: 1500.0,
                password_hash: "".to_string(),
                is_bot: false,
                is_gagged: false,
                is_mod: true,
                is_admin: true,
                is_banned: false,
            }),
            "test_gagged" => Ok(Player {
                id: 2,
                username: "test_gagged".into(),
                email: "test_gagged@example.com".into(),
                rating: 1200.0,
                password_hash: "".to_string(),
                is_bot: false,
                is_gagged: true,
                is_mod: false,
                is_admin: false,
                is_banned: false,
            }),
            _ => ServiceError::not_found("Player not found"),
        }
    }

    fn validate_login(&self, _username: &PlayerUsername, _password: &str) -> ServiceResult<()> {
        Ok(())
    }

    fn try_login(
        &self,
        _id: &ClientId,
        _username: &PlayerUsername,
        _password: &str,
    ) -> ServiceResult<()> {
        Ok(())
    }

    fn try_login_jwt(&self, _id: &ClientId, _token: &str) -> ServiceResult<PlayerUsername> {
        Ok("".to_string())
    }

    fn try_login_guest(
        &self,
        _id: &ClientId,
        _token: Option<&str>,
    ) -> ServiceResult<PlayerUsername> {
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
