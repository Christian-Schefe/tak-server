use tak_server_domain::{
    ServiceError,
    player::{Player, PlayerFilter, PlayerUsername},
    transport::ListenerId,
};

use crate::protocol::v2::{ProtocolV2Handler, ProtocolV2Result, V2Response, split_n_and_rest};

impl ProtocolV2Handler {
    pub async fn handle_sudo_message(
        &self,
        id: ListenerId,
        username: &PlayerUsername,
        msg: &str,
        parts: &[&str],
    ) -> ProtocolV2Result {
        if parts.len() < 2 {
            return ServiceError::bad_request("Invalid Sudo command format");
        }
        let command = parts[1];
        self.send_to(id, format!("sudoReply > {}", msg));
        let response = match command {
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
            "reload" => Ok(V2Response::OK), // Was used in legacy profanity filter, no-op here.
            "broadcast" => Ok(V2Response::OK), // What's the point, players can already broadcast via global chat anyways.
            "set" => self.handle_set_message(username, parts).await,
            _ => ServiceError::bad_request("Unknown Sudo command"),
        };
        match response {
            Ok(V2Response::Message(msg)) => Ok(V2Response::Message(format!("sudoReply {}", msg))),
            Ok(V2Response::ErrorMessage(e, msg)) => {
                Ok(V2Response::ErrorMessage(e, format!("sudoReply {}", msg)))
            }
            Ok(V2Response::OK) => Ok(V2Response::Message("sudoReply Success".to_string())),
            Err(e) => {
                let err_str = format!("sudoReply {}", e);
                Ok(V2Response::ErrorMessage(e, err_str))
            }
        }
    }

    async fn handle_player_update(
        &self,
        username: &PlayerUsername,
        parts: &[&str],
        silenced: Option<bool>,
        modded: Option<bool>,
        admin: Option<bool>,
        bot: Option<bool>,
    ) -> ProtocolV2Result {
        if parts.len() != 3 {
            return ServiceError::bad_request("Invalid Sudo command format");
        }
        let target_username = parts[2].to_string();
        if let Some(silenced) = silenced {
            self.app
                .player_service
                .set_silenced(username, &target_username, silenced)
                .await?;

            return Ok(V2Response::Message(format!(
                "{} {}",
                target_username,
                if silenced { "gagged" } else { "ungagged" }
            )));
        } else if let Some(modded) = modded {
            self.app
                .player_service
                .set_modded(username, &target_username, modded)
                .await?;

            return Ok(V2Response::Message(format!(
                "{} {} as moderator",
                if modded { "Added" } else { "Removed" },
                target_username
            )));
        } else if let Some(admin) = admin {
            self.app
                .player_service
                .set_admin(username, &target_username, admin)
                .await?;

            return Ok(V2Response::Message(format!(
                "{} {} as admin",
                if admin { "Added" } else { "Removed" },
                target_username
            )));
        } else if let Some(bot) = bot {
            self.app
                .player_service
                .set_bot(username, &target_username, bot)
                .await?;

            return Ok(V2Response::Message(format!(
                "{} {} as bot",
                if bot { "Added" } else { "Removed" },
                target_username
            )));
        } else {
            return ServiceError::bad_request("No valid player update specified");
        }
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
        self.app
            .player_service
            .try_kick(username, &target_username)
            .await?;
        Ok(V2Response::Message(format!("{} kicked", target_username)))
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

        self.app
            .player_service
            .set_banned(username, &target_username, ban_msg)
            .await?;
        Ok(V2Response::Message(format!(
            "{} {}",
            target_username,
            if ban { "banned" } else { "unbanned" }
        )))
    }

    async fn handle_list_message(&self, parts: &[&str]) -> ProtocolV2Result {
        if parts.len() != 3 {
            return ServiceError::bad_request("Invalid Sudo list command format");
        }
        let list_type = parts[2];

        let player_service = &self.app.player_service;

        let player_filter = match list_type {
            "ban" => PlayerFilter {
                is_banned: Some(true),
                ..Default::default()
            },
            "gag" => PlayerFilter {
                is_silenced: Some(true),
                ..Default::default()
            },
            "mod" => PlayerFilter {
                is_mod: Some(true),
                ..Default::default()
            },
            "admin" => PlayerFilter {
                is_admin: Some(true),
                ..Default::default()
            },
            "bot" => PlayerFilter {
                is_bot: Some(true),
                ..Default::default()
            },
            _ => {
                return ServiceError::bad_request("Unknown Sudo list command");
            }
        };

        let players: Vec<(_, Player)> = player_service.get_players(player_filter).await?.players;
        let response = format!(
            "[{}]",
            players
                .into_iter()
                .map(|(_, p)| p.username)
                .collect::<Vec<_>>()
                .join(", ")
        );
        Ok(V2Response::Message(response))
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
                self.app
                    .player_service
                    .set_password(username, &target_username, value)
                    .await?;
                Ok(V2Response::Message("Password set".to_string()))
            }
            _ => ServiceError::bad_request("Unknown Sudo set setting"),
        }
    }
}
