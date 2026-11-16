use std::time::Duration;

use chrono::{DateTime, Utc};
use tak_core::{
    TakAsyncTimeControl, TakBaseGameSettings, TakGameSettings, TakOpening, TakRealtimeTimeControl,
    TakReserve, TakTimeSettings,
};

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum GameStatusResponseType {
    Ongoing {
        requests: ForPlayerResponse<GameRequestsResponse>,
    },
    Finished {
        result: String,
    },
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameStatusResponse {
    #[serde(flatten)]
    pub metadata: GameMetadataResponse,
    pub actions: Vec<String>,
    pub remaining_ms: ForPlayerResponse<u64>,
    pub status: GameStatusResponseType,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameRecordResponse {
    #[serde(flatten)]
    pub metadata: GameMetadataResponse,
    pub rating_info: Option<GameRatingInfoResponse>,
    pub result: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameRequestsResponse {
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
pub enum GameRequestResponse {
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
pub enum GameRequestResponseType {
    Draw,
    Undo,
    MoreTime,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ForPlayerResponse<R> {
    pub white: R,
    pub black: R,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameMetadataResponse {
    pub id: String,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub date: DateTime<Utc>,
    pub white: PlayerSnapshotResponse,
    pub black: PlayerSnapshotResponse,
    pub is_rated: bool,
    pub game_settings: GameSettingsResponse,
    pub match_id: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameRatingInfoResponse {
    pub rating_change: ForPlayerResponse<f64>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlayerSnapshotResponse {
    pub user_id: String,
    pub username: String,
    pub rating: Option<f64>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BaseGameSettingsResponse {
    pub board_size: u32,
    pub half_komi: u32,
    pub pieces: u32,
    pub capstones: u32,
    pub opening: TakOpeningResponse,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum TakOpeningResponse {
    Swap,
    NoSwap,
    DoubleStack,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameSettingsResponse {
    #[serde(flatten)]
    pub base: BaseGameSettingsResponse,
    pub time_settings: TimeSettingsResponse,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(
    rename_all = "camelCase",
    tag = "type",
    rename_all_fields = "camelCase"
)]
pub enum TimeSettingsResponse {
    Realtime {
        contingent_ms: u64,
        increment_ms: u64,
        extra: Option<ExtraTimeResponse>,
    },
    Async {
        contingent_ms: u64,
    },
}

impl BaseGameSettingsResponse {
    pub fn from_base_settings(settings: &TakBaseGameSettings) -> Self {
        BaseGameSettingsResponse {
            board_size: settings.board_size,
            half_komi: settings.half_komi,
            pieces: settings.reserve.pieces,
            capstones: settings.reserve.capstones,
            opening: match settings.opening {
                TakOpening::Swap => TakOpeningResponse::Swap,
                TakOpening::NoSwap => TakOpeningResponse::NoSwap,
                TakOpening::DoubleStack => TakOpeningResponse::DoubleStack,
            },
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
            opening: match self.opening {
                TakOpeningResponse::Swap => TakOpening::Swap,
                TakOpeningResponse::NoSwap => TakOpening::NoSwap,
                TakOpeningResponse::DoubleStack => TakOpening::DoubleStack,
            },
        }
    }
}

impl GameSettingsResponse {
    pub fn from_game_settings(settings: &TakGameSettings) -> Self {
        GameSettingsResponse {
            base: BaseGameSettingsResponse::from_base_settings(&settings.base),
            time_settings: match &settings.time_settings {
                TakTimeSettings::Realtime(tc) => TimeSettingsResponse::Realtime {
                    contingent_ms: tc.contingent.as_millis() as u64,
                    increment_ms: tc.increment.as_millis() as u64,
                    extra: tc.extra.map(|(on_move, extra_time)| ExtraTimeResponse {
                        on_move,
                        extra_ms: extra_time.as_millis() as u64,
                    }),
                },
                TakTimeSettings::Async(tc) => TimeSettingsResponse::Async {
                    contingent_ms: tc.contingent.as_millis() as u64,
                },
            },
        }
    }

    pub fn to_game_settings(&self) -> TakGameSettings {
        TakGameSettings {
            base: self.base.to_base_settings(),
            time_settings: match &self.time_settings {
                TimeSettingsResponse::Realtime {
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
                TimeSettingsResponse::Async {
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
pub struct ExtraTimeResponse {
    pub on_move: u32,
    pub extra_ms: u64,
}
