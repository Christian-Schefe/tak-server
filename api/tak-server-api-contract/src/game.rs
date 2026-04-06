use std::time::Duration;

use chrono::{DateTime, Utc};
use tak_core::{
    TakAsyncTimeControl, TakBaseGameSettings, TakGameSettings, TakRealtimeTimeControl, TakReserve,
    TakTimeSettings,
};

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RequestResponse {
    pub accept: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameStatus {
    pub id: i64,
    pub match_id: Option<i64>,
    pub player_ids: ForPlayer<String>,
    pub is_rated: bool,
    pub game_settings: GameSettingsInfo,
    pub actions: Vec<String>,
    pub status: GameStatusType,
    pub remaining_ms: ForPlayer<u64>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameRequest {
    pub id: u64,
    pub from_player_id: String,
    pub request_type: JsonGameRequestType,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(
    rename_all = "camelCase",
    tag = "type",
    rename_all_fields = "camelCase"
)]
pub enum JsonGameRequestType {
    Draw,
    Undo,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(
    rename_all = "camelCase",
    tag = "type",
    rename_all_fields = "camelCase"
)]
pub enum GameStatusType {
    Ongoing { requests: Vec<GameRequest> },
    Ended { result: String },
    Aborted,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ForPlayer<R> {
    pub white: R,
    pub black: R,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct JsonGameMetadata {
    pub id: i64,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub date: DateTime<Utc>,
    pub player_ids: ForPlayer<String>,
    pub is_rated: bool,
    pub game_settings: GameSettingsInfo,
}

#[derive(serde::Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct JsonEndedGameInfo {
    pub white: JsonPlayerSnapshot,
    pub black: JsonPlayerSnapshot,
    pub rating_info: Option<JsonGameRatingInfo>,
    pub result: Option<String>,
    #[serde(flatten)]
    pub metadata: JsonGameMetadata,
}

#[derive(serde::Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct JsonGameRatingInfo {
    pub rating_change: ForPlayer<f64>,
}

#[derive(serde::Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct JsonPlayerSnapshot {
    pub username: Option<String>,
    pub rating: Option<f64>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameSettingsInfoBase {
    pub board_size: u32,
    pub half_komi: u32,
    pub pieces: u32,
    pub capstones: u32,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameSettingsInfo {
    #[serde(flatten)]
    pub base: GameSettingsInfoBase,
    pub time_settings: JsonTimeSettings,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(
    rename_all = "camelCase",
    tag = "type",
    rename_all_fields = "camelCase"
)]
pub enum JsonTimeSettings {
    Realtime {
        contingent_ms: u64,
        increment_ms: u64,
        extra: Option<ExtraTime>,
    },
    Async {
        contingent_ms: u64,
    },
}

impl GameSettingsInfoBase {
    pub fn from_base_settings(settings: &TakBaseGameSettings) -> Self {
        GameSettingsInfoBase {
            board_size: settings.board_size,
            half_komi: settings.half_komi,
            pieces: settings.reserve.pieces,
            capstones: settings.reserve.capstones,
        }
    }

    pub fn to_base_settings(&self) -> TakBaseGameSettings {
        TakBaseGameSettings {
            board_size: self.board_size,
            half_komi: self.half_komi,
            reserve: TakReserve {
                pieces: self.pieces,
                capstones: self.capstones,
            },
        }
    }
}

impl GameSettingsInfo {
    pub fn from_game_settings(settings: &TakGameSettings) -> Self {
        GameSettingsInfo {
            base: GameSettingsInfoBase::from_base_settings(&settings.base),
            time_settings: match &settings.time_settings {
                TakTimeSettings::Realtime(tc) => JsonTimeSettings::Realtime {
                    contingent_ms: tc.contingent.as_millis() as u64,
                    increment_ms: tc.increment.as_millis() as u64,
                    extra: tc.extra.map(|(on_move, extra_time)| ExtraTime {
                        on_move,
                        extra_ms: extra_time.as_millis() as u64,
                    }),
                },
                TakTimeSettings::Async(tc) => JsonTimeSettings::Async {
                    contingent_ms: tc.contingent.as_millis() as u64,
                },
            },
        }
    }

    pub fn to_game_settings(&self) -> TakGameSettings {
        TakGameSettings {
            base: self.base.to_base_settings(),
            time_settings: match &self.time_settings {
                JsonTimeSettings::Realtime {
                    contingent_ms,
                    increment_ms,
                    extra,
                } => TakTimeSettings::Realtime(TakRealtimeTimeControl {
                    contingent: Duration::from_millis(*contingent_ms),
                    increment: Duration::from_millis(*increment_ms),
                    extra: extra
                        .as_ref()
                        .map(|extra| (extra.on_move, Duration::from_millis(extra.extra_ms))),
                }),
                JsonTimeSettings::Async {
                    contingent_ms: increment_ms,
                } => TakTimeSettings::Async(TakAsyncTimeControl {
                    contingent: Duration::from_millis(*increment_ms),
                }),
            },
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExtraTime {
    pub on_move: u32,
    pub extra_ms: u64,
}
