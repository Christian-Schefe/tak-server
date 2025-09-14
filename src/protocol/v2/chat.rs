use crate::{
    chat::{
        join_room, leave_room, send_message_to_all, send_message_to_player, send_message_to_room,
    },
    client::{ClientId, send_to},
    protocol::{ChatMessageSource, ServerMessage},
};

pub fn handle_server_chat_message(id: &ClientId, msg: &ServerMessage) {
    match msg {
        ServerMessage::ChatMessage {
            from,
            message,
            source,
        } => {
            let msg = match source {
                ChatMessageSource::Global => format!("Shout <{}> {}", from, message),
                ChatMessageSource::Room(room) => {
                    format!("ShoutRoom {} <{}> {}", room, from, message)
                }
                ChatMessageSource::Private => format!("Tell <{}> {}", from, message),
            };
            send_to(id, msg);
        }
        ServerMessage::ConfirmPrivateMessage { to, message } => {
            send_to(id, format!("Told <{}> {}", to, message));
        }
        ServerMessage::RoomMembership { room, joined } => {
            let msg = if *joined {
                format!("Joined room {}", room)
            } else {
                format!("Left room {}", room)
            };
            send_to(id, msg);
        }
        _ => {
            eprintln!("Unhandled server chat message: {:?}", msg);
        }
    }
}

pub fn handle_room_membership_message(id: &ClientId, parts: &[&str], join: bool) {
    if parts.len() != 2 {
        send_to(id, "NOK");
        return;
    }
    let room = parts[1];
    if join {
        join_room(id, room);
    } else {
        leave_room(id, room);
    }
}

pub fn handle_shout_message(id: &ClientId, msg: &str) {
    let msg = msg.replacen("Shout ", "", 1);
    if msg.is_empty() {
        send_to(id, "NOK");
        return;
    }
    if let Err(e) = send_message_to_all(id, &msg) {
        println!("Error handling Shout message: {}", e);
        send_to(id, "NOK");
    }
}

pub fn handle_shout_room_message(id: &ClientId, parts: &[&str], msg: &str) {
    if parts.len() < 3 {
        send_to(id, "NOK");
        return;
    }
    let room = parts[1];
    let msg = msg.replacen(&format!("ShoutRoom {} ", room), "", 1);
    if msg.is_empty() {
        send_to(id, "NOK");
        return;
    }
    if let Err(e) = send_message_to_room(id, room, &msg) {
        println!("Error handling Shout message: {}", e);
        send_to(id, "NOK");
    }
}

pub fn handle_tell_message(id: &ClientId, parts: &[&str], msg: &str) {
    if parts.len() < 3 {
        send_to(id, "NOK");
        return;
    }
    let target_username = parts[1];
    let msg = msg.replacen(&format!("Tell {} ", target_username), "", 1);
    if msg.is_empty() {
        send_to(id, "NOK");
        return;
    }
    if let Err(e) = send_message_to_player(id, &target_username.to_string(), &msg) {
        println!("Error handling Tell message: {}", e);
        send_to(id, "NOK");
    }
}
