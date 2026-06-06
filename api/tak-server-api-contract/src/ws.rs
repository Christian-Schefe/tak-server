use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    game::{ForPlayer, JsonGameMetadata, JsonGameRequest},
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
        game_id: String,
        action: String,
    },
    ChatMessage {
        message: String,
        conversation: JsonChatConversation,
    },
    SpectateGame {
        game_id: String,
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
        seek_id: String,
    },
    GameEvent {
        game_id: String,
        #[serde(flatten)]
        event_type: ServerGameEventType,
        time_info: ForPlayer<u64>,
    },
    GameStarted {
        game: JsonGameMetadata,
    },
    GameEnded {
        game_id: String,
    },
    ChatMessage {
        message: JsonChatMessage,
        conversation: JsonChatConversation,
    },
    MatchEvent {
        match_id: String,
        #[serde(flatten)]
        event_type: ServerMatchEventType,
    },
    AccountsOnline {
        account_ids: Vec<String>,
    },
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(
    tag = "eventType",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ServerMatchEventType {
    ReadinessChanged { player_id: Option<String> },
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct JsonChatMessage {
    pub message_id: i64,
    pub sender: String,
    pub message: String,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub timestamp: DateTime<Utc>,
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
    GameRequestChanged {
        player_id: String,
        request: JsonGameRequest,
    },
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum JsonChatConversation {
    Global,
    Room {
        room_name: String,
    },
    Private {
        account_id1: String,
        account_id2: String,
    },
}
