use std::sync::{Arc, LazyLock};

use dashmap::DashMap;

use crate::{
    client::{
        ClientId, get_associated_client, get_associated_player, try_protocol_broadcast,
        try_protocol_multicast, try_protocol_send,
    },
    player::PlayerUsername,
    protocol::{ChatMessageSource, ServerMessage},
};

static CHAT_ROOMS: LazyLock<Arc<DashMap<String, Vec<ClientId>>>> =
    LazyLock::new(|| Arc::new(DashMap::new()));

pub fn join_room(client_id: &ClientId, room_name: &str) {
    let mut room = CHAT_ROOMS.entry(room_name.to_string()).or_default();
    if !room.contains(client_id) {
        room.push(*client_id);
    }
    let msg = ServerMessage::RoomMembership {
        room: room_name.to_string(),
        joined: true,
    };
    try_protocol_send(client_id, &msg);
}

pub fn leave_room(client_id: &ClientId, room_name: &str) {
    if let Some(mut room) = CHAT_ROOMS.get_mut(room_name) {
        room.retain(|id| id != client_id);
        if room.is_empty() {
            CHAT_ROOMS.remove(room_name);
        }
    }
    let msg = ServerMessage::RoomMembership {
        room: room_name.to_string(),
        joined: false,
    };
    try_protocol_send(client_id, &msg);
}

pub fn send_message_to_all(client_id: &ClientId, message: &str) -> Result<(), String> {
    let Some(username) = get_associated_player(client_id) else {
        return Err("Client not associated with a player".to_string());
    };
    let msg = ServerMessage::ChatMessage {
        from: username,
        message: message.to_string(),
        source: ChatMessageSource::Global,
    };
    try_protocol_broadcast(&msg);
    Ok(())
}

pub fn send_message_to_room(
    client_id: &ClientId,
    room_name: &str,
    message: &str,
) -> Result<(), String> {
    let Some(username) = get_associated_player(client_id) else {
        return Err("Client not associated with a player".to_string());
    };
    if let Some(room) = CHAT_ROOMS.get(room_name) {
        let msg = ServerMessage::ChatMessage {
            from: username,
            message: message.to_string(),
            source: ChatMessageSource::Room(room_name.to_string()),
        };
        try_protocol_multicast(&room, &msg);
    }
    Ok(())
}

pub fn send_message_to_player(
    from_client_id: &ClientId,
    to_username: &PlayerUsername,
    message: &str,
) -> Result<(), String> {
    let Some(from_username) = get_associated_player(from_client_id) else {
        return Err("Client not associated with a player".to_string());
    };
    let Some(to_client_id) = get_associated_client(to_username) else {
        return Ok(());
    };
    let msg = ServerMessage::ChatMessage {
        from: from_username,
        message: message.to_string(),
        source: ChatMessageSource::Private,
    };
    try_protocol_send(&to_client_id, &msg);
    Ok(())
}
