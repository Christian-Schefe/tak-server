use std::time::Instant;

use crate::{
    ArcChatService, ArcClientService, ArcGameService, ArcPlayerService, ArcSeekService,
    ServiceError, ServiceResult,
    client::ClientId,
    player::PlayerUsername,
    protocol::{ServerGameMessage, ServerMessage},
};

mod auth;
mod chat;
mod game;
mod game_list;
mod seek;
mod sudo;

pub struct ProtocolV2Handler {
    client_service: ArcClientService,
    seek_service: ArcSeekService,
    player_service: ArcPlayerService,
    chat_service: ArcChatService,
    game_service: ArcGameService,
}

pub type ProtocolV2Result = ServiceResult<Option<String>>;

impl ProtocolV2Handler {
    pub fn new(
        client_service: ArcClientService,
        seek_service: ArcSeekService,
        player_service: ArcPlayerService,
        chat_service: ArcChatService,
        game_service: ArcGameService,
    ) -> Self {
        Self {
            client_service,
            seek_service,
            player_service,
            chat_service,
            game_service,
        }
    }

    pub fn handle_client_message(&self, id: &ClientId, msg: String) {
        let parts = msg.split_whitespace().collect::<Vec<_>>();
        if parts.is_empty() {
            println!("Received empty message");
            return;
        }
        let res: ProtocolV2Result = match parts[0].to_ascii_lowercase().as_str() {
            "ping" => Ok(None),
            "protocol" => Ok(None), // Noop, ignore
            "client" => Ok(None),   // Noop, ignore
            "quit" | "exit" => {
                self.client_service.close_client(id);
                Ok(None)
            }
            "login" => self.handle_login_message(id, &parts),
            "logintoken" => self.handle_login_token_message(id, &parts),
            "register" => self.handle_register_message(&parts),
            "sendresettoken" => self.handle_reset_token_message(&parts),
            "resetpassword" => self.handle_reset_password_message(&parts),
            _ => self.handle_logged_in_client_message(id, &parts, &msg),
        };
        match res {
            Ok(Some(msg)) => {
                self.send_to(id, msg);
            }
            Ok(None) => {
                self.send_to(id, "OK");
            }
            Err(e) => {
                println!("Error handling message {:?}: {}", parts, e);
                self.send_to(id, "NOK");
            }
        }
    }

    pub fn handle_server_message(&self, id: &ClientId, msg: &ServerMessage) {
        match msg {
            ServerMessage::SeekList { add, seek_id } => {
                self.handle_server_seek_list_message(id, seek_id, *add);
            }
            ServerMessage::GameMessage { game_id, message } => {
                self.handle_server_game_message(id, game_id, message);
            }
            ServerMessage::PlayersOnline { players } => {
                let online_message = format!("Online {}", players.len());
                let players_message =
                    format!("OnlinePlayers {}", serde_json::to_string(players).unwrap());
                self.send_to(id, online_message);
                self.send_to(id, players_message);
            }
            ServerMessage::AcceptRematch { seek_id } => {
                let rematch_message = format!("Accept Rematch {}", seek_id);
                self.send_to(id, rematch_message);
            }
            ServerMessage::GameList { .. } | ServerMessage::GameStart { .. } => {
                self.handle_server_game_list_message(id, msg);
            }
            ServerMessage::ChatMessage { .. } | ServerMessage::RoomMembership { .. } => {
                self.handle_server_chat_message(id, msg);
            }
        }
    }

    pub fn on_authenticated(&self, id: &ClientId, username: &PlayerUsername) {
        let seek_ids = self.seek_service.get_seek_ids();
        for seek_id in seek_ids {
            let seek_msg = ServerMessage::SeekList { add: true, seek_id };
            self.handle_server_message(id, &seek_msg);
        }
        let game_ids = self.game_service.get_game_ids();
        for game_id in game_ids {
            let game_msg = ServerMessage::GameList { add: true, game_id };
            self.handle_server_message(id, &game_msg);
        }
        if let Some(active_game) = self.game_service.get_active_game_of_player(username) {
            let start_msg = ServerMessage::GameStart {
                game_id: active_game.id,
            };
            self.handle_server_message(id, &start_msg);
            for action in &active_game.game.action_history {
                let action_msg = ServerMessage::GameMessage {
                    game_id: active_game.id,
                    message: ServerGameMessage::Action(action.clone()),
                };
                self.handle_server_message(id, &action_msg);
            }
            let now = Instant::now();
            let remaining = active_game.game.get_time_remaining_both(now);
            let time_msg = ServerMessage::GameMessage {
                game_id: active_game.id,
                message: ServerGameMessage::TimeUpdate { remaining },
            };
            self.handle_server_message(id, &time_msg);
        }
    }

    fn handle_logged_in_client_message(
        &self,
        id: &ClientId,
        parts: &[&str],
        msg: &str,
    ) -> ProtocolV2Result {
        let Some(username) = self.client_service.get_associated_player(id) else {
            return ServiceError::unauthorized("Client is not logged in");
        };

        match parts[0].to_ascii_lowercase().as_str() {
            "changepassword" => self.handle_change_password_message(&username, &parts),
            "seek" => self.handle_seek_message(&username, &parts),
            "rematch" => self.handle_rematch_message(&username, &parts),
            "list" => self.handle_seek_list_message(id),
            "gamelist" => self.handle_game_list_message(id),
            "accept" => self.handle_accept_message(&username, &parts),
            "observe" => self.handle_observe_message(id, &parts, true),
            "unobserve" => self.handle_observe_message(id, &parts, false),
            "shout" => self.handle_shout_message(&username, &msg),
            "shoutroom" => self.handle_shout_room_message(&username, &msg),
            "tell" => self.handle_tell_message(&username, &msg),
            "joinroom" => self.handle_room_membership_message(id, &parts, true),
            "leaveroom" => self.handle_room_membership_message(id, &parts, false),
            "sudo" => self.handle_sudo_message(&username, msg, &parts),
            s if s.starts_with("game#") => self.handle_game_message(&username, &parts),
            _ => ServiceError::bad_request(format!("Unknown V2 message type: {}", parts[0])),
        }
    }

    fn send_to<T>(&self, id: &ClientId, msg: T)
    where
        T: AsRef<str>,
    {
        crate::client::send_to(&**self.client_service, id, msg);
    }
}

fn split_n_and_rest(input: &str, n: usize) -> (Vec<&str>, &str) {
    let mut parts = Vec::new();
    let mut last_pos = 0;

    // Use split_whitespace to find the first n tokens
    for (i, part) in input.split_whitespace().enumerate() {
        if i < n {
            // Find this token's position in the original string
            if let Some(pos) = input[last_pos..].find(part) {
                last_pos += pos + part.len();
            }
            parts.push(part);
        } else {
            break;
        }
    }

    // Slice from the last consumed position to keep the remainder intact
    let remainder = if last_pos < input.len() {
        &input[last_pos..] // includes original spaces
    } else {
        ""
    };

    (parts, remainder.trim_start()) // trim leading spaces in remainder
}
