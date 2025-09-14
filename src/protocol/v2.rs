use crate::{
    client::{ClientId, send_to},
    player::{login_guest, try_login},
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
        "Login" => handle_login_message(*id, &parts),
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
        ServerMessage::GameAction { .. }
        | ServerMessage::GameOver { .. }
        | ServerMessage::GameDrawOffer { .. }
        | ServerMessage::GameUndoRequest { .. }
        | ServerMessage::GameUndo { .. }
        | ServerMessage::GameTimeUpdate { .. } => {
            game::handle_game_server_message(id, msg);
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
        ServerMessage::ChatMessage { .. }
        | ServerMessage::ConfirmPrivateMessage { .. }
        | ServerMessage::RoomMembership { .. } => {
            chat::handle_server_chat_message(id, msg);
        }
    }
}

fn handle_login_message(id: ClientId, parts: &[&str]) {
    if parts.len() >= 2 && parts[1] == "Guest" {
        let token = parts.get(2).copied();
        login_guest(&id, token);
        return;
    }
    if parts.len() != 3 {
        send_to(&id, "NOK");
    }
    let username = parts[1].to_string();
    let password = parts[2].to_string();

    if !try_login(&id, &username, &password) {
        println!("Login failed for user: {}", id);
        send_to(&id, "NOK");
    } else {
        send_to(&id, format!("Welcome {}!", username));
    }
}
