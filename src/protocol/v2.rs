use crate::{
    client::{ClientId, send_to},
    player::{login_guest, reset_password, try_login, try_login_jwt, try_register},
    protocol::ServerMessage,
};

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
        "Login" => handle_login_message(id, &parts),
        "LoginToken" => handle_login_token_message(id, &parts),
        "Register" => handle_register_message(id, &parts),
        "ResetPassword" => handle_reset_password_message(id),
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

fn handle_login_message(id: &ClientId, parts: &[&str]) {
    if parts.len() >= 2 && parts[1] == "Guest" {
        let token = parts.get(2).copied();
        login_guest(id, token);
        return;
    }
    if parts.len() != 3 {
        send_to(id, "NOK");
    }
    let username = parts[1].to_string();
    let password = parts[2].to_string();

    if let Err(e) = try_login(id, &username, &password) {
        println!("Login failed for user {}: {}", id, e);
        send_to(id, "NOK");
    } else {
        send_to(id, format!("Welcome {}!", username));
    }
}

fn handle_login_token_message(id: &ClientId, parts: &[&str]) {
    if parts.len() != 2 {
        send_to(id, "NOK");
        return;
    }
    let token = parts[1];
    match try_login_jwt(id, token) {
        Ok(username) => {
            send_to(id, format!("Welcome {}!", username));
        }
        Err(e) => {
            println!("Login with token failed for user {}: {}", id, e);
            send_to(id, "NOK");
        }
    }
}

fn handle_register_message(id: &ClientId, parts: &[&str]) {
    if parts.len() != 3 {
        send_to(id, "NOK");
        return;
    }
    let username = parts[1].to_string();
    let email = parts[2].to_string();

    if let Err(e) = try_register(&username, &email) {
        println!("Error registering user {}: {}", username, e);
        send_to(id, format!("Registration Error: {}", e));
    } else {
        send_to(
            id,
            format!(
                "Registered {}. Check your email for the temporary password",
                username
            ),
        );
    }
}

fn handle_reset_password_message(id: &ClientId) {
    if let Err(e) = reset_password(id) {
        println!("Error resetting password for client {}: {}", id, e);
        send_to(id, format!("Password Reset Error: {}", e));
    } else {
        send_to(
            id,
            "Password reset. Check your email for the temporary password.",
        );
    }
}
