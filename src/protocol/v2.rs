use std::time::Instant;

use crate::{
    ArcChatService, ArcClientService, ArcGameService, ArcPlayerService, ArcSeekService,
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
        match parts[0] {
            "PING" => {
                self.send_to(id, "OK");
            }
            "Login" => self.handle_login_message(id, &parts),
            "LoginToken" => self.handle_login_token_message(id, &parts),
            "Register" => self.handle_register_message(id, &parts),
            "SendResetToken" => self.handle_reset_token_message(id, &parts),
            "ResetPassword" => self.handle_reset_password_message(id, &parts),
            _ => self.handle_logged_in_client_message(id, &parts, &msg),
        };
    }

    pub fn handle_server_message(&self, id: &ClientId, msg: &ServerMessage) {
        match msg {
            ServerMessage::SeekList { add, seek } => {
                self.handle_server_seek_list_message(id, &seek, *add);
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
            ServerMessage::GameList { .. }
            | ServerMessage::GameStart { .. }
            | ServerMessage::ObserveGame { .. } => {
                self.handle_server_game_list_message(id, msg);
            }
            ServerMessage::ChatMessage { .. } | ServerMessage::RoomMembership { .. } => {
                self.handle_server_chat_message(id, msg);
            }
        }
    }

    pub fn on_authenticated(&self, id: &ClientId, username: &PlayerUsername) {
        let seeks = self.seek_service.get_seeks();
        for seek in seeks {
            let seek_msg = ServerMessage::SeekList { add: true, seek };
            self.handle_server_message(id, &seek_msg);
        }
        let games = self.game_service.get_games();
        for game in games {
            let game_msg = ServerMessage::GameList { add: true, game };
            self.handle_server_message(id, &game_msg);
        }
        if let Some(active_game) = self.game_service.get_active_game_of_player(username) {
            let start_msg = ServerMessage::GameStart {
                game: active_game.clone(),
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

    fn handle_logged_in_client_message(&self, id: &ClientId, parts: &[&str], msg: &str) {
        let Some(username) = self.client_service.get_associated_player(id) else {
            println!("Client {} is not logged in", id);
            self.send_to(id, "NOK");
            return;
        };

        match parts[0] {
            "ChangePassword" => self.handle_change_password_message(id, &username, &parts),
            "Seek" => self.handle_seek_message(id, &username, &parts),
            "Rematch" => self.handle_rematch_message(id, &username, &parts),
            "List" => self.handle_seek_list_message(id),
            "GameList" => self.handle_game_list_message(id),
            "Accept" => self.handle_accept_message(id, &username, &parts),
            "Observe" => self.handle_observe_message(id, &parts, true),
            "Unobserve" => self.handle_observe_message(id, &parts, false),
            "Shout" => self.handle_shout_message(id, &username, &msg),
            "ShoutRoom" => self.handle_shout_room_message(id, &username, &parts, &msg),
            "Tell" => self.handle_tell_message(id, &username, &parts, &msg),
            "JoinRoom" => self.handle_room_membership_message(id, &parts, true),
            "LeaveRoom" => self.handle_room_membership_message(id, &parts, false),
            "Sudo" => self.handle_sudo_message(id, &username, &parts),
            s if s.starts_with("Game#") => self.handle_game_message(id, &parts),
            _ => {
                println!("Unknown V2 message type: {}", parts[0]);
                self.send_to(id, "NOK");
            }
        };
    }

    fn send_to<T>(&self, id: &ClientId, msg: T)
    where
        T: AsRef<str>,
    {
        crate::client::send_to(&**self.client_service, id, msg);
    }
}
