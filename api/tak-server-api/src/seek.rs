use axum::{Json, extract::State};
use tak_core::TakPlayer;
use tak_server_app::workflow::matchmaking::SeekView;

use crate::AppState;

pub async fn get_seeks(State(app): State<AppState>) -> Json<Vec<SeekInfo>> {
    let seeks = app.app.seek_list_use_case.list_seeks();
    Json(
        seeks
            .into_iter()
            .map(|seek| SeekInfo::from_seek_view(seek))
            .collect(),
    )
}

#[derive(serde::Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SeekInfo {
    pub id: u64,
    pub creator_id: String,
    pub opponent_id: Option<String>,
    pub color: String,
    pub board_size: u32,
    pub half_komi: u32,
    pub pieces: u32,
    pub capstones: u32,
    pub contingent_ms: u64,
    pub increment_ms: u64,
    pub extra: Option<ExtraTime>,
    pub is_rated: bool,
}

impl SeekInfo {
    pub fn from_seek_view(seek: SeekView) -> Self {
        SeekInfo {
            id: seek.id.0 as u64,
            creator_id: seek.creator_id.to_string(),
            opponent_id: seek.opponent_id.map(|id| id.to_string()),
            color: match seek.color {
                None => "random".to_string(),
                Some(TakPlayer::White) => "white".to_string(),
                Some(TakPlayer::Black) => "black".to_string(),
            },
            board_size: seek.game_settings.board_size as u32,
            half_komi: seek.game_settings.half_komi as u32,
            pieces: seek.game_settings.reserve.pieces as u32,
            capstones: seek.game_settings.reserve.capstones as u32,
            contingent_ms: seek.game_settings.time_control.contingent.as_millis() as u64,
            increment_ms: seek.game_settings.time_control.increment.as_millis() as u64,
            extra: seek
                .game_settings
                .time_control
                .extra
                .map(|(on_move, extra_time)| ExtraTime {
                    on_move: on_move as u32,
                    extra_ms: extra_time.as_millis() as u64,
                }),
            is_rated: seek.is_rated,
        }
    }
}

#[derive(serde::Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExtraTime {
    pub on_move: u32,
    pub extra_ms: u64,
}
