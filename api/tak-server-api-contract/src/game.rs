use std::time::Duration;

use chrono::{DateTime, Utc};
use tak_core::{
    TakAsyncTimeControl, TakBaseGameSettings, TakGameSettings, TakRealtimeTimeControl, TakReserve,
    TakTimeSettings,
};

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct JsonGameStatus {
    pub id: String,
    pub match_id: Option<String>,
    pub player_ids: ForPlayer<String>,
    pub is_rated: bool,
    pub game_settings: JsonGameSettings,
    pub actions: Vec<String>,
    pub status: GameStatusType,
    pub remaining_ms: ForPlayer<u64>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct JsonGameRequests {
    pub draw_offered: bool,
    pub undo_requested: bool,
    pub more_time_offered: Option<u64>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(
    rename_all = "camelCase",
    tag = "type",
    rename_all_fields = "camelCase"
)]
pub enum JsonGameRequest {
    Draw { offer: bool },
    Undo { request: bool },
    MoreTime { amount_ms: Option<u64> },
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
    MoreTime,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(
    rename_all = "camelCase",
    tag = "type",
    rename_all_fields = "camelCase"
)]
pub enum GameStatusType {
    Ongoing {
        white_requests: JsonGameRequests,
        black_requests: JsonGameRequests,
    },
    Ended {
        result: String,
    },
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
    pub id: String,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub date: DateTime<Utc>,
    pub player_ids: ForPlayer<String>,
    pub is_rated: bool,
    pub game_settings: JsonGameSettings,
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
pub struct JsonBaseGameSettings {
    pub board_size: u32,
    pub half_komi: u32,
    pub pieces: u32,
    pub capstones: u32,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct JsonGameSettings {
    #[serde(flatten)]
    pub base: JsonBaseGameSettings,
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

impl JsonBaseGameSettings {
    pub fn from_base_settings(settings: &TakBaseGameSettings) -> Self {
        JsonBaseGameSettings {
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

impl JsonGameSettings {
    pub fn from_game_settings(settings: &TakGameSettings) -> Self {
        JsonGameSettings {
            base: JsonBaseGameSettings::from_base_settings(&settings.base),
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
