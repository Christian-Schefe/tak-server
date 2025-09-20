use crate::{client::ClientId, player::PlayerUsername, protocol::v2::ProtocolV2Handler};

impl ProtocolV2Handler {
    pub fn handle_sudo_message(&self, id: &ClientId, username: &PlayerUsername, parts: &[&str]) {
        if parts.len() < 2 {
            eprintln!("Sudo command requires at least one argument");
            self.send_to(id, "NOK");
            return;
        }
        let command = parts[1];
        match command {
            "ban" => self.handle_ban_message(id, username, parts, true),
            "unban" => self.handle_ban_message(id, username, parts, false),
            "gag" => self.handle_player_update(username, parts, Some(true), None, None, None),
            "ungag" => self.handle_player_update(username, parts, Some(false), None, None, None),
            "mod" => self.handle_player_update(username, parts, None, Some(true), None, None),
            "unmod" => self.handle_player_update(username, parts, None, Some(false), None, None),
            "admin" => self.handle_player_update(username, parts, None, None, Some(true), None),
            "unadmin" => self.handle_player_update(username, parts, None, None, Some(false), None),
            "bot" => self.handle_player_update(username, parts, None, None, None, Some(true)),
            "unbot" => self.handle_player_update(username, parts, None, None, None, Some(false)),
            "kick" => self.client_service.close_client(id),
            "list" => self.handle_list_message(id, parts),
            "reload" => {}    // Was used in legacy profanity filter, no-op here.
            "broadcast" => {} // What's the point, players can already broadcast via global chat anyways.
            "set" => self.handle_set_message(id, username, parts),
            _ => {
                eprintln!("Unknown Sudo command: {}", command);
            }
        }
    }

    fn handle_player_update(
        &self,
        username: &PlayerUsername,
        parts: &[&str],
        gagged: Option<bool>,
        modded: Option<bool>,
        admin: Option<bool>,
        bot: Option<bool>,
    ) {
        if parts.len() != 3 {
            eprintln!("Invalid Sudo {} command format: {:?}", parts[1], parts);
            return;
        }
        let target_username = parts[2].to_string();
        if let Some(gagged) = gagged {
            if let Err(e) = self
                .player_service
                .set_gagged(username, &target_username, gagged)
            {
                eprintln!(
                    "Failed to set gagged={} for user {}: {}",
                    gagged, target_username, e
                );
            }
        }
        if let Some(modded) = modded {
            if let Err(e) = self
                .player_service
                .set_modded(username, &target_username, modded)
            {
                eprintln!(
                    "Failed to set modded={} for user {}: {}",
                    modded, target_username, e
                );
            }
        }
        if let Some(admin) = admin {
            if let Err(e) = self
                .player_service
                .set_admin(username, &target_username, admin)
            {
                eprintln!(
                    "Failed to set admin={} for user {}: {}",
                    admin, target_username, e
                );
            }
        }
        if let Some(bot) = bot {
            if let Err(e) = self.player_service.set_bot(username, &target_username, bot) {
                eprintln!(
                    "Failed to set bot={} for user {}: {}",
                    bot, target_username, e
                );
            }
        }
    }

    fn handle_ban_message(
        &self,
        id: &ClientId,
        username: &PlayerUsername,
        parts: &[&str],
        ban: bool,
    ) {
        if parts.len() < 4 {
            eprintln!("Invalid Sudo {} command format: {:?}", parts[1], parts);
            return;
        }
        let target_username = parts[2].to_string();
        let ban_msg = if ban {
            Some(parts[3..].join(" "))
        } else {
            None
        };

        if let Err(e) = self
            .player_service
            .set_banned(id, username, &target_username, ban_msg)
        {
            eprintln!(
                "Failed to set banned=true for user {}: {}",
                target_username, e
            );
        }
    }

    fn handle_list_message(&self, id: &ClientId, parts: &[&str]) {
        if parts.len() != 3 {
            eprintln!("Invalid Sudo list command format: {:?}", parts);
            self.send_to(id, "NOK");
            return;
        }
        let list_type = parts[2];

        let players = match list_type {
            "ban" => self
                .player_service
                .get_players(Some(true), None, None, None, None),
            "gag" => self
                .player_service
                .get_players(None, Some(true), None, None, None),
            "mod" => self
                .player_service
                .get_players(None, None, Some(true), None, None),
            "admin" => self
                .player_service
                .get_players(None, None, None, Some(true), None),
            "bot" => self
                .player_service
                .get_players(None, None, None, None, Some(true)),
            _ => {
                eprintln!("Unknown Sudo list type: {}", list_type);
                self.send_to(id, "NOK");
                return;
            }
        };
        let players = match players {
            Ok(players) => players,
            Err(e) => {
                eprintln!("Failed to get player list for {}: {}", list_type, e);
                self.send_to(id, "NOK");
                return;
            }
        };
        let response = format!(
            "[{}]",
            players
                .into_iter()
                .map(|p| p.username)
                .collect::<Vec<_>>()
                .join(", ")
        );
        self.send_to(id, &response);
    }

    fn handle_set_message(&self, id: &ClientId, username: &PlayerUsername, parts: &[&str]) {
        if parts.len() != 5 {
            eprintln!("Invalid Sudo set command format: {:?}", parts);
            self.send_to(id, "NOK");
            return;
        }
        let setting = parts[2];
        let target_username = parts[3].to_string();
        let value = parts[4];

        match setting {
            "password" => {
                if let Err(e) = self
                    .player_service
                    .set_password(username, &target_username, value)
                {
                    eprintln!("Failed to set password for {}: {}", username, e);
                    self.send_to(id, "NOK");
                    return;
                }
                self.send_to(id, "OK");
            }
            _ => {
                eprintln!("Unknown Sudo set setting: {}", setting);
                self.send_to(id, "NOK");
            }
        }
    }
}
