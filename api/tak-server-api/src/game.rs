use axum::{Json, extract::State};
use tak_core::TakGameSettings;
use tak_server_app::workflow::gameplay::GameMetadataView;

use crate::AppState;

pub async fn get_games(State(app): State<AppState>) -> Json<Vec<GameInfo>> {
    let games = app.app.game_list_ongoing_use_case.list_games();
    Json(
        games
            .into_iter()
            .map(|game| GameInfo::from_ongoing_game_view(&game.metadata))
            .collect(),
    )
}

#[derive(serde::Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameInfo {
    pub id: i64,
    pub white_id: String,
    pub black_id: String,
    pub is_rated: bool,
    #[serde(flatten)]
    pub settings: GameSettingsInfo,
}

impl GameInfo {
    pub fn from_ongoing_game_view(view: &GameMetadataView) -> Self {
        GameInfo {
            id: view.id.0,
            white_id: view.white_id.to_string(),
            black_id: view.black_id.to_string(),
            is_rated: view.is_rated,
            settings: GameSettingsInfo::from_game_settings(&view.settings),
        }
    }
}

#[derive(serde::Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameSettingsInfo {
    pub board_size: u32,
    pub half_komi: u32,
    pub pieces: u32,
    pub capstones: u32,
    pub contingent_ms: u64,
    pub increment_ms: u64,
    pub extra: Option<ExtraTime>,
}

impl GameSettingsInfo {
    pub fn from_game_settings(settings: &TakGameSettings) -> Self {
        GameSettingsInfo {
            board_size: settings.board_size,
            half_komi: settings.half_komi,
            pieces: settings.reserve.pieces,
            capstones: settings.reserve.capstones,
            contingent_ms: settings.time_control.contingent.as_millis() as u64,
            increment_ms: settings.time_control.increment.as_millis() as u64,
            extra: settings
                .time_control
                .extra
                .map(|(on_move, extra_time)| ExtraTime {
                    on_move,
                    extra_ms: extra_time.as_millis() as u64,
                }),
        }
    }
}

#[derive(serde::Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExtraTime {
    pub on_move: u32,
    pub extra_ms: u64,
}
