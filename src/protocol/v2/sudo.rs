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
            "gag" => self.handle_player_update(username, parts, Some(true), None, None, None, None),
            "ungag" => {
                self.handle_player_update(username, parts, Some(false), None, None, None, None)
            }
            // TODO: ban message
            "ban" => self.handle_player_update(username, parts, None, Some(true), None, None, None),
            "unban" => {
                self.handle_player_update(username, parts, None, Some(false), None, None, None)
            }
            "mod" => self.handle_player_update(username, parts, None, None, Some(true), None, None),
            "unmod" => {
                self.handle_player_update(username, parts, None, None, Some(false), None, None)
            }
            "admin" => {
                self.handle_player_update(username, parts, None, None, None, Some(true), None)
            }
            "unadmin" => {
                self.handle_player_update(username, parts, None, None, None, Some(false), None)
            }
            "bot" => self.handle_player_update(username, parts, None, None, None, None, Some(true)),
            "unbot" => {
                self.handle_player_update(username, parts, None, None, None, None, Some(false))
            }
            // TODO: more sudo commands
            "kick" => {}
            "list" => {}
            "reload" => {}
            "broadcast" => {}
            "set" => {}
            _ => {
                eprintln!("Unknown Sudo command: {}", command);
            }
        }
    }

    pub fn handle_player_update(
        &self,
        username: &PlayerUsername,
        parts: &[&str],
        gagged: Option<bool>,
        banned: Option<bool>,
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
        if let Some(banned) = banned {
            if let Err(e) = self
                .player_service
                .set_banned(username, &target_username, banned)
            {
                eprintln!(
                    "Failed to set banned={} for user {}: {}",
                    banned, target_username, e
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
}
