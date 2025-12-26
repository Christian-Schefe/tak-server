use std::{sync::Arc, time::Instant};

use log::info;
use tak_server_app::{
    domain::{ListenerId, PlayerId},
    ports::notification::ListenerMessage,
};

use crate::{
    app::ServiceError,
    client::{DisconnectReason, ServerMessage, TransportServiceImpl},
    protocol::Application,
};

mod auth;
mod chat;
mod game;
mod game_list;
mod seek;
mod sudo;

pub struct ProtocolV2Handler {
    app: Arc<Application>,
    transport: TransportServiceImpl,
}

pub enum V2Response {
    OK,
    Message(String),
    ErrorMessage(ServiceError, String),
    ErrorNOK(ServiceError),
}

impl ProtocolV2Handler {
    pub fn new(app: &Arc<Application>, transport: &TransportServiceImpl) -> Self {
        Self {
            app: app.clone(),
            transport: transport.clone(),
        }
    }

    pub async fn handle_client_message(&self, id: ListenerId, msg: String) {
        let parts = msg.split_whitespace().collect::<Vec<_>>();
        if parts.is_empty() {
            info!("Received empty message");
            return;
        }
        let res: V2Response = match parts[0].to_ascii_lowercase().as_str() {
            "ping" => V2Response::OK,
            "protocol" => V2Response::OK, // Noop, ignore
            "client" => V2Response::OK,   // Noop, ignore
            "quit" | "exit" => {
                self.transport
                    .close_with_reason(id, DisconnectReason::ClientQuit)
                    .await;
                V2Response::OK
            }
            "login" => self.handle_login_message(id, &parts).await,
            "register" => self.handle_register_message(id, &parts).await,
            "sendresettoken" => self.handle_reset_token_message(id, &parts).await,
            "resetpassword" => self.handle_reset_password_message(&parts).await,
            _ => self.handle_logged_in_client_message(id, &parts, &msg).await,
        };
        match res {
            V2Response::Message(msg) => {
                self.send_to(id, msg);
            }
            V2Response::OK => {
                self.send_to(id, "OK");
            }
            V2Response::ErrorMessage(err, msg) => {
                info!("Error handling message {:?}: {}", parts, err);
                self.send_to(id, msg);
            }
            V2Response::ErrorNOK(err) => {
                info!("Error handling message {:?}: {}", parts, err);
                self.send_to(id, "NOK");
            }
        }
    }
    pub async fn handle_server_message(&self, id: ListenerId, msg: &ServerMessage) {
        match msg {
            ServerMessage::ConnectionClosed { reason } => {
                let reason_str = match reason {
                    DisconnectReason::NewSession => {
                        "You've logged in from another window. Disconnecting"
                    }
                    DisconnectReason::Inactivity => "Disconnected due to inactivity",
                    DisconnectReason::Ban(msg) => &msg,
                    DisconnectReason::Kick => "You have been kicked from the server",
                    DisconnectReason::ServerShutdown => "Server is shutting down",
                    DisconnectReason::ClientQuit => "Quitting. Goodbye!",
                };
                let disconnect_message = format!("Message {}", reason_str);
                self.send_to(id, disconnect_message);
            }
            notif => self.handle_notification_message(id, notif).await,
        }
    }

    async fn handle_notification_message(&self, id: ListenerId, msg: &ListenerMessage) {
        match msg {
            ListenerMessage::SeekCreated { seek } => {
                self.handle_server_seek_list_message(id, seek, true).await;
            }
            ListenerMessage::SeekCanceled { seek } => {
                self.handle_server_seek_list_message(id, seek, false).await;
            }
            ListenerMessage::GameMessage { game_id, message } => {
                self.handle_server_game_message(id, game_id, message);
            }
            ListenerMessage::PlayersOnline { players } => {
                let online_message = format!("Online {}", players.len());
                let players_message =
                    format!("OnlinePlayers {}", serde_json::to_string(players).unwrap());
                self.send_to(id, online_message);
                self.send_to(id, players_message);
            }
            ListenerMessage::AcceptRematch { seek_id } => {
                let rematch_message = format!("Accept Rematch {}", seek_id);
                self.send_to(id, rematch_message);
            }
            ListenerMessage::GameList { .. } | ListenerMessage::GameStart { .. } => {
                self.handle_server_game_list_message(id, msg).await;
            }
            ListenerMessage::ChatMessage { .. } => {
                self.handle_server_chat_message(id, msg);
            }
        }
    }

    pub async fn on_authenticated(&self, id: ListenerId, player_id: PlayerId) {
        let seeks = self.app.seek_service.get_seeks();
        for seek in seeks {
            let seek_msg = ServerMessage::Notification(ListenerMessage::SeekCreated { seek });
            self.handle_server_message(id, &seek_msg).await;
        }
        let games = self.app.game_service.get_games();
        for game in games {
            let game_msg = ServerMessage::GameList { add: true, game };
            self.handle_server_message(id, &game_msg).await;
        }
        if let Some(active_game) = self.app.game_service.get_active_game_of_player(player_id) {
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

    pub fn on_connected(&self, id: ListenerId) {
        self.send_to(id, "Welcome!");
        self.send_to(id, "Login or Register");
    }

    async fn handle_logged_in_client_message(
        &self,
        id: ListenerId,
        parts: &[&str],
        msg: &str,
    ) -> V2Response {
        let Some(player_id) = self.transport.get_associated_player(id) else {
            return V2Response::ErrorNOK(ServiceError::Unauthorized(
                "Client is not logged in".to_string(),
            ));
        };

        match parts[0].to_ascii_lowercase().as_str() {
            "changepassword" => {
                self.handle_change_password_message(&player_id, &parts)
                    .await
            }
            "seek" => self.handle_seek_message(player_id, &parts).await,
            "rematch" => self.handle_rematch_message(player_id, &parts).await,
            "list" => self.handle_seek_list_message(id).await,
            "gamelist" => self.handle_game_list_message(id),
            "accept" => self.handle_accept_message(player_id, &parts).await,
            "observe" => self.handle_observe_message(id, &parts, true),
            "unobserve" => self.handle_observe_message(id, &parts, false),
            "shout" => self.handle_shout_message(player_id, &msg).await,
            "shoutroom" => self.handle_shout_room_message(player_id, &msg).await,
            "tell" => self.handle_tell_message(player_id, &msg).await,
            "joinroom" => self.handle_room_membership_message(id, &parts, true).await,
            "leaveroom" => self.handle_room_membership_message(id, &parts, false).await,
            "sudo" => self.handle_sudo_message(id, player_id, msg, &parts).await,
            s if s.starts_with("game#") => self.handle_game_message(player_id, &parts).await,
            _ => ServiceError::BadRequest(format!("Unknown V2 message type: {}", parts[0])),
        }
    }

    fn send_to<T>(&self, id: ListenerId, msg: T)
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
