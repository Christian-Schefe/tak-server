use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    game::{ForPlayerResponse, GameMetadataResponse, GameRequestResponse},
    seek::SeekResponse,
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
        conversation: ChatConversationResponse,
    },
    SpectateGame {
        game_id: String,
        spectate: bool,
    },
    SpectateMatch {
        match_id: String,
        spectate: bool,
    },
    SpectateChatConversation {
        conversation: ChatConversationResponse,
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
        seek: SeekResponse,
    },
    SeekRemoved {
        seek_id: String,
    },
    GameEvent {
        game_id: String,
        #[serde(flatten)]
        event_type: ServerGameEventType,
        time_info: ForPlayerResponse<u64>,
    },
    GameStarted {
        game: GameMetadataResponse,
    },
    GameEnded {
        game_id: String,
    },
    ChatMessage {
        message: ChatMessageResponse,
        conversation: ChatConversationResponse,
    },
    MatchEvent {
        match_id: String,
        #[serde(flatten)]
        event_type: ServerMatchEventType,
    },
    UsersOnline {
        user_ids: Vec<String>,
    },
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(
    tag = "eventType",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ServerMatchEventType {
    ReadinessChanged { user_id: Option<String> },
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessageResponse {
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
        player: String,
        request: GameRequestResponse,
    },
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ChatConversationResponse {
    Global,
    Room { room_name: String },
    Private { user_id1: String, user_id2: String },
}
