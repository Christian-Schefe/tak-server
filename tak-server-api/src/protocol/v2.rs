use std::time::Instant;

use tak_server_domain::{
    ServiceError, ServiceResult,
    app::AppState,
    player::PlayerUsername,
    transport::{DisconnectReason, ServerGameMessage, ServerMessage},
};

use crate::client::{ClientId, TransportServiceImpl};

mod auth;
mod chat;
mod game;
mod game_list;
mod seek;
mod sudo;

pub struct ProtocolV2Handler {
    app_state: AppState,
    transport: TransportServiceImpl,
}

pub type ProtocolV2Result = ServiceResult<Option<String>>;

impl ProtocolV2Handler {
    pub fn new(app: &AppState, transport: &TransportServiceImpl) -> Self {
        Self {
            app_state: app.clone(),
            transport: transport.clone(),
        }
    }

    pub async fn handle_client_message(&self, id: &ClientId, msg: String) {
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
                self.transport.close_client(id);
                Ok(None)
            }
            "login" => self.handle_login_message(id, &parts).await,
            "logintoken" => self.handle_login_token_message(id, &parts).await,
            "register" => self.handle_register_message(id, &parts).await,
            "sendresettoken" => self.handle_reset_token_message(id, &parts).await,
            "resetpassword" => self.handle_reset_password_message(&parts).await,
            _ => self.handle_logged_in_client_message(id, &parts, &msg).await,
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

    pub async fn handle_server_message(&self, id: &ClientId, msg: &ServerMessage) {
        match msg {
            ServerMessage::SeekList { add, seek } => {
                self.handle_server_seek_list_message(id, seek, *add).await;
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
                self.handle_server_game_list_message(id, msg).await;
            }
            ServerMessage::ChatMessage { .. } | ServerMessage::RoomMembership { .. } => {
                self.handle_server_chat_message(id, msg);
            }
            ServerMessage::ConnectionClosed { reason } => {
                let reason_str = match reason {
                    DisconnectReason::NewSession => {
                        "You've logged in from another window. Disconnecting"
                    }
                    DisconnectReason::Inactivity => "Disconnected due to inactivity",
                    DisconnectReason::Ban(msg) => &msg,
                    DisconnectReason::Kick => "You have been kicked from the server",
                };
                let disconnect_message = format!("Message {}", reason_str);
                self.send_to(id, disconnect_message);
            }
        }
    }

    pub async fn on_authenticated(&self, id: &ClientId, username: &PlayerUsername) {
        let seeks = self.app_state.seek_service.get_seeks();
        for seek in seeks {
            let seek_msg = ServerMessage::SeekList { add: true, seek };
            self.handle_server_message(id, &seek_msg).await;
        }
        let games = self.app_state.game_service.get_games();
        for game in games {
            let game_msg = ServerMessage::GameList { add: true, game };
            self.handle_server_message(id, &game_msg).await;
        }
        if let Some(active_game) = self
            .app_state
            .game_service
            .get_active_game_of_player(username)
        {
            let start_msg = ServerMessage::GameStart {
                game_id: active_game.id,
            };
            self.handle_server_message(id, &start_msg).await;
            for action in &active_game.game.action_history {
                let action_msg = ServerMessage::GameMessage {
                    game_id: active_game.id,
                    message: ServerGameMessage::Action {
                        action: action.clone(),
                    },
                };
                self.handle_server_message(id, &action_msg).await;
            }
            let now = Instant::now();
            let (remaining_white, remaining_black) = active_game.game.get_time_remaining_both(now);
            let time_msg = ServerMessage::GameMessage {
                game_id: active_game.id,
                message: ServerGameMessage::TimeUpdate {
                    remaining_white,
                    remaining_black,
                },
            };
            self.handle_server_message(id, &time_msg).await;
        }
    }

    pub fn on_connected(&self, id: &ClientId) {
        self.send_to(id, "Welcome!");
        self.send_to(id, "Login or Register");
    }

    async fn handle_logged_in_client_message(
        &self,
        id: &ClientId,
        parts: &[&str],
        msg: &str,
    ) -> ProtocolV2Result {
        let Some(username) = self.transport.get_associated_player(id) else {
            return ServiceError::unauthorized("Client is not logged in");
        };

        match parts[0].to_ascii_lowercase().as_str() {
            "changepassword" => {
                self.handle_change_password_message(id, &username, &parts)
                    .await
            }
            "seek" => self.handle_seek_message(&username, &parts).await,
            "rematch" => self.handle_rematch_message(&username, &parts).await,
            "list" => self.handle_seek_list_message(id).await,
            "gamelist" => self.handle_game_list_message(id),
            "accept" => self.handle_accept_message(&username, &parts).await,
            "observe" => self.handle_observe_message(id, &parts, true),
            "unobserve" => self.handle_observe_message(id, &parts, false),
            "shout" => self.handle_shout_message(&username, &msg).await,
            "shoutroom" => self.handle_shout_room_message(&username, &msg).await,
            "tell" => self.handle_tell_message(&username, &msg).await,
            "joinroom" => self.handle_room_membership_message(id, &parts, true).await,
            "leaveroom" => self.handle_room_membership_message(id, &parts, false).await,
            "sudo" => self.handle_sudo_message(&username, msg, &parts).await,
            s if s.starts_with("game#") => self.handle_game_message(&username, &parts).await,
            _ => ServiceError::bad_request(format!("Unknown V2 message type: {}", parts[0])),
        }
    }

    fn send_to<T>(&self, id: &ClientId, msg: T)
    where
        T: AsRef<str>,
    {
        let _ = self.transport.try_send_to(id, msg.as_ref());
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
