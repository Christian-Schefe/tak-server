use tak_server_domain::{ServiceError, player::PlayerUsername};

use crate::protocol::v2::{ProtocolV2Handler, ProtocolV2Result, split_n_and_rest};

impl ProtocolV2Handler {
    pub async fn handle_sudo_message(
        &self,
        username: &PlayerUsername,
        msg: &str,
        parts: &[&str],
    ) -> ProtocolV2Result {
        if parts.len() < 2 {
            return ServiceError::bad_request("Invalid Sudo command format");
        }
        let command = parts[1];
        match command {
            "ban" => self.handle_ban_message(username, msg, true).await,
            "unban" => self.handle_ban_message(username, msg, false).await,
            "gag" => {
                self.handle_player_update(username, parts, Some(true), None, None, None)
                    .await
            }
            "ungag" => {
                self.handle_player_update(username, parts, Some(false), None, None, None)
                    .await
            }
            "mod" => {
                self.handle_player_update(username, parts, None, Some(true), None, None)
                    .await
            }
            "unmod" => {
                self.handle_player_update(username, parts, None, Some(false), None, None)
                    .await
            }
            "admin" => {
                self.handle_player_update(username, parts, None, None, Some(true), None)
                    .await
            }
            "unadmin" => {
                self.handle_player_update(username, parts, None, None, Some(false), None)
                    .await
            }
            "bot" => {
                self.handle_player_update(username, parts, None, None, None, Some(true))
                    .await
            }
            "unbot" => {
                self.handle_player_update(username, parts, None, None, None, Some(false))
                    .await
            }
            "kick" => self.handle_kick_message(username, parts).await,
            "list" => self.handle_list_message(parts).await,
            "reload" => Ok(None), // Was used in legacy profanity filter, no-op here.
            "broadcast" => Ok(None), // What's the point, players can already broadcast via global chat anyways.
            "set" => self.handle_set_message(username, parts).await,
            _ => ServiceError::bad_request("Unknown Sudo command"),
        }
    }

    async fn handle_player_update(
        &self,
        username: &PlayerUsername,
        parts: &[&str],
        gagged: Option<bool>,
        modded: Option<bool>,
        admin: Option<bool>,
        bot: Option<bool>,
    ) -> ProtocolV2Result {
        if parts.len() != 3 {
            return ServiceError::bad_request("Invalid Sudo command format");
        }
        let target_username = parts[2].to_string();
        if let Some(gagged) = gagged {
            self.app_state
                .player_service
                .set_gagged(username, &target_username, gagged)
                .await?;
        }
        if let Some(modded) = modded {
            self.app_state
                .player_service
                .set_modded(username, &target_username, modded)
                .await?;
        }
        if let Some(admin) = admin {
            self.app_state
                .player_service
                .set_admin(username, &target_username, admin)
                .await?;
        }
        if let Some(bot) = bot {
            self.app_state
                .player_service
                .set_bot(username, &target_username, bot)
                .await?;
        }
        Ok(None)
    }

    async fn handle_kick_message(
        &self,
        username: &PlayerUsername,
        parts: &[&str],
    ) -> ProtocolV2Result {
        if parts.len() != 3 {
            return ServiceError::bad_request("Invalid Sudo kick command format");
        }
        let target_username = parts[2].to_string();
        self.app_state
            .player_service
            .try_kick(username, &target_username)
            .await?;
        Ok(None)
    }

    async fn handle_ban_message(
        &self,
        username: &PlayerUsername,
        orig_msg: &str,
        ban: bool,
    ) -> ProtocolV2Result {
        let (parts, msg) = split_n_and_rest(orig_msg, 3);
        if parts.len() != 3 {
            return ServiceError::bad_request("Invalid Ban/Unban message format");
        }
        let target_username = parts[2].to_string();
        let ban_msg = if ban { Some(msg.to_string()) } else { None };

        self.app_state
            .player_service
            .set_banned(username, &target_username, ban_msg)
            .await?;
        Ok(None)
    }

    async fn handle_list_message(&self, parts: &[&str]) -> ProtocolV2Result {
        if parts.len() != 3 {
            return ServiceError::bad_request("Invalid Sudo list command format");
        }
        let list_type = parts[2];

        let player_service = &self.app_state.player_service;

        let players = match list_type {
            "ban" => {
                player_service
                    .get_players(Some(true), None, None, None, None)
                    .await?
            }
            "gag" => {
                player_service
                    .get_players(None, Some(true), None, None, None)
                    .await?
            }
            "mod" => {
                player_service
                    .get_players(None, None, Some(true), None, None)
                    .await?
            }
            "admin" => {
                player_service
                    .get_players(None, None, None, Some(true), None)
                    .await?
            }
            "bot" => {
                player_service
                    .get_players(None, None, None, None, Some(true))
                    .await?
            }
            _ => {
                return ServiceError::bad_request("Unknown Sudo list type");
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
        Ok(Some(response))
    }

    async fn handle_set_message(
        &self,
        username: &PlayerUsername,
        parts: &[&str],
    ) -> ProtocolV2Result {
        if parts.len() != 5 {
            return ServiceError::bad_request("Invalid Sudo set command format");
        }
        let setting = parts[2];
        let target_username = parts[3].to_string();
        let value = parts[4];

        match setting {
            "password" => {
                self.app_state
                    .player_service
                    .set_password(username, &target_username, value)
                    .await?;
            }
            _ => {
                return ServiceError::bad_request("Unknown Sudo set setting");
            }
        }
        Ok(None)
    }
}
