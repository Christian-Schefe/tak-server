use uuid::Uuid;

use crate::{
    game::{ForPlayer, JsonGameMetadata, JsonGameRequestType},
    seek::SeekInfo,
};

#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ClientMessage {
    Authenticate {
        token: String,
    },
    GameAction {
        game_id: i64,
        action: String,
    },
    ChatMessage {
        message: String,
        target: JsonChatMessageTarget,
    },
    SpectateGame {
        game_id: i64,
        spectate: bool,
    },
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ClientMessageWrapper {
    #[serde(flatten)]
    pub message: ClientMessage,
    pub response_id: Uuid,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ServerMessage {
    Success {
        response_id: Uuid,
    },
    Error {
        message: String,
        code: u16,
        response_id: Uuid,
    },
    SeekCreated {
        seek: SeekInfo,
    },
    SeekRemoved {
        seek_id: u64,
    },
    GameEvent {
        game_id: i64,
        #[serde(flatten)]
        event_type: ServerGameEventType,
        time_info: ForPlayer<u64>,
    },
    GameStarted {
        game: JsonGameMetadata,
    },
    GameEnded {
        game_id: i64,
    },
    ChatMessage {
        from_account_id: String,
        message: String,
        target: JsonChatMessageTarget,
    },
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(
    tag = "eventType",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ServerGameEventType {
    GameAction {
        ply_index: usize,
        action: String,
    },
    GameActionUndone {
        ply_index: usize,
    },
    GameEnded {
        result: String,
    },
    GameRequestAdded {
        request_id: u64,
        request_type: JsonGameRequestType,
        from_player_id: String,
    },
    GameRequestRemoved {
        request_id: u64,
    },
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum JsonChatMessageTarget {
    Global,
    Room { room_name: String },
    Private { to_account_id: String },
}
