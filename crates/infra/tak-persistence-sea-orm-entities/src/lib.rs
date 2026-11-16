use std::collections::HashMap;

use sea_orm::{DeriveValueType, FromJsonQueryResult, sea_query::ValueTypeErr};

pub mod account;
pub mod chat;
pub mod game;
pub mod game_history;
pub mod game_state;
pub mod matches;
pub mod moderation;
pub mod profile;
pub mod rating;
pub mod rating_history;
pub mod stats;
pub mod tournament;
pub mod tournament_players;
pub mod tournament_round;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, FromJsonQueryResult)]
#[serde(rename_all = "camelCase")]
pub struct JsonGameSettings {
    pub board_size: u32,
    pub half_komi: u32,
    pub pieces: u32,
    pub capstones: u32,
    pub opening: JsonTakOpening,
    pub time_settings: JsonTimeSettings,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum JsonTakPlayer {
    White,
    Black,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum JsonTakOpening {
    Swap,
    NoSwap,
    DoubleStack,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum JsonTimeSettings {
    Realtime(JsonRealtimeTimeSettings),
    Async(JsonAsyncTimeSettings),
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonRealtimeTimeSettings {
    pub contingent_ms: u64,
    pub increment_ms: u64,
    pub extra: Option<JsonRealtimeTimeExtra>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonRealtimeTimeExtra {
    pub extra_time_ms: u64,
    pub extra_time_move: u32,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonAsyncTimeSettings {
    pub increment_ms: u64,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum JsonMatchMode {
    Unlimited,
    FixedGames(u32),
    FirstTo(u32),
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, FromJsonQueryResult)]
pub struct JsonMatchSettings {
    pub game_settings: JsonGameSettings,
    pub match_mode: JsonMatchMode,
    pub is_rated: bool,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, DeriveValueType)]
#[sea_orm(value_type = "String")]
pub enum JsonMatchStatus {
    Waiting,
    Ongoing,
    Completed,
}

impl std::str::FromStr for JsonMatchStatus {
    type Err = ValueTypeErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Waiting" => Ok(JsonMatchStatus::Waiting),
            "Ongoing" => Ok(JsonMatchStatus::Ongoing),
            "Completed" => Ok(JsonMatchStatus::Completed),
            _ => Err(ValueTypeErr),
        }
    }
}

impl std::fmt::Display for JsonMatchStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            JsonMatchStatus::Waiting => "Waiting",
            JsonMatchStatus::Ongoing => "Ongoing",
            JsonMatchStatus::Completed => "Completed",
        };
        write!(f, "{}", s)
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonTimeInfo {
    pub white_remaining_ms: u64,
    pub black_remaining_ms: u64,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, FromJsonQueryResult)]
#[serde(rename_all = "camelCase")]
pub struct JsonGameEvent {
    pub remaining_ms: JsonTimeInfo,
    pub event_type: JsonGameEventType,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum JsonGameEventType {
    Action {
        action: String,
        player: JsonTakPlayer,
        ply_index: u32,
    },
    Undo {
        ply_index: u32,
    },
    GameOver {
        result: String,
    },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, FromJsonQueryResult)]
#[serde(rename_all = "camelCase")]
pub struct JsonClock {
    pub remaining_time: JsonTimeInfo,
    pub is_ticking: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, FromJsonQueryResult)]
#[serde(rename_all = "camelCase")]
pub struct JsonGameRequests {
    pub draw_offered: bool,
    pub undo_requested: bool,
    pub more_time_offered: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, FromJsonQueryResult)]
#[serde(rename_all = "camelCase")]
pub struct JsonHistory(pub Vec<JsonActionRecord>);

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonActionRecord {
    pub action: String,
    pub time_info: JsonTimeInfo,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonResponse {
    pub responses: HashMap<String, Option<JsonResponseEntry>>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonResponseEntry {
    pub action: String,
    pub response: JsonResponse,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, FromJsonQueryResult)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum JsonTournamentFormat {
    Swiss { rounds: u32 },
    RoundRobin,
    GroupRoundRobin { group_size: u32 },
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, FromJsonQueryResult)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum JsonTournamentStatus {
    Upcoming { registration_open: bool },
    Ongoing,
    Completed,
}
