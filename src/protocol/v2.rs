use std::time::Instant;

use crate::{
    client::{ClientId, send_to},
    game::{get_active_game_of_player, get_games},
    player::PlayerUsername,
    protocol::{ServerGameMessage, ServerMessage},
    seek::get_seeks,
};

mod auth;
mod chat;
mod game;
mod game_list;
mod seek;

pub fn handle_client_message(id: &ClientId, msg: String) {
    let parts = msg.split_whitespace().collect::<Vec<_>>();
    if parts.is_empty() {
        println!("Received empty message");
        return;
    }
    match parts[0] {
        "PING" => {
            send_to(id, "OK");
        }
        "Login" => auth::handle_login_message(id, &parts),
        "LoginToken" => auth::handle_login_token_message(id, &parts),
        "Register" => auth::handle_register_message(id, &parts),
        "SendResetToken" => auth::handle_reset_token_message(id, &parts),
        "ResetPassword" => auth::handle_reset_password_message(id, &parts),
        "ChangePassword" => auth::handle_change_password_message(id, &parts),
        "Seek" => seek::handle_seek_message(id, &parts),
        "Accept" => seek::handle_accept_message(id, &parts),
        "Observe" => game_list::handle_observe_message(id, &parts, true),
        "Unobserve" => game_list::handle_observe_message(id, &parts, false),
        "Shout" => chat::handle_shout_message(id, &msg),
        "ShoutRoom" => chat::handle_shout_room_message(id, &parts, &msg),
        "Tell" => chat::handle_tell_message(id, &parts, &msg),
        "JoinRoom" => chat::handle_room_membership_message(id, &parts, true),
        "LeaveRoom" => chat::handle_room_membership_message(id, &parts, false),
        s if s.starts_with("Game#") => game::handle_game_message(id, &parts),
        _ => {
            println!("Unknown V2 message type: {}", parts[0]);
            send_to(id, "NOK");
        }
    };
}

pub fn handle_server_message(id: &ClientId, msg: &ServerMessage) {
    match msg {
        ServerMessage::SeekList { add, seek } => {
            seek::send_seek_list_message(id, &seek, *add);
        }
        ServerMessage::GameMessage { game_id, message } => {
            game::handle_game_server_message(id, game_id, message);
        }
        ServerMessage::PlayersOnline { players } => {
            let online_message = format!("Online {}", players.len());
            let players_message =
                format!("OnlinePlayers {}", serde_json::to_string(players).unwrap());
            send_to(id, online_message);
            send_to(id, players_message);
        }
        ServerMessage::GameList { .. }
        | ServerMessage::GameStart { .. }
        | ServerMessage::ObserveGame { .. } => {
            game_list::handle_server_game_list_message(id, msg);
        }
        ServerMessage::ChatMessage { .. } | ServerMessage::RoomMembership { .. } => {
            chat::handle_server_chat_message(id, msg);
        }
    }
}

pub fn on_authenticated(id: &ClientId, username: &PlayerUsername) {
    let seeks = get_seeks();
    for seek in seeks {
        let seek_msg = ServerMessage::SeekList { add: true, seek };
        handle_server_message(id, &seek_msg);
    }
    let games = get_games();
    for game in games {
        let game_msg = ServerMessage::GameList { add: true, game };
        handle_server_message(id, &game_msg);
    }
    if let Some(active_game) = get_active_game_of_player(username) {
        let start_msg = ServerMessage::GameStart {
            game: active_game.clone(),
        };
        handle_server_message(id, &start_msg);
        for action in &active_game.game.action_history {
            let action_msg = ServerMessage::GameMessage {
                game_id: active_game.id,
                message: ServerGameMessage::Action(action.clone()),
            };
            handle_server_message(id, &action_msg);
        }
        let now = Instant::now();
        let remaining = active_game.game.get_time_remaining_both(now);
        let time_msg = ServerMessage::GameMessage {
            game_id: active_game.id,
            message: ServerGameMessage::TimeUpdate { remaining },
        };
        handle_server_message(id, &time_msg);
    }
}
