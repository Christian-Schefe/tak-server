use axum::{
    Json,
    extract::{Path, State},
};
use tak_core::{TakGameSettings, ptn::action_to_ptn};
use tak_server_app::{domain::GameId, workflow::gameplay::GameMetadataView};

use crate::{AppState, ServiceError};

pub async fn get_games(State(app): State<AppState>) -> Json<Vec<GameInfo>> {
    let games = app.app.game_list_ongoing_use_case.list_games();
    Json(
        games
            .into_iter()
            .map(|game| GameInfo::from_ongoing_game_view(&game.metadata))
            .collect(),
    )
}

pub async fn get_ongoing_game_status(
    State(app): State<AppState>,
    Path(game_id): Path<i64>,
) -> Result<Json<OngoingGameStatus>, ServiceError> {
    let game = app
        .app
        .game_get_ongoing_use_case
        .get_game(GameId(game_id))
        .ok_or_else(|| {
            ServiceError::NotFound(format!("Ongoing game with id {} not found", game_id))
        })?;

    Ok(Json(OngoingGameStatus {
        id: game.metadata.id.0,
        actions: game
            .game
            .action_history()
            .iter()
            .map(|a| action_to_ptn(&a.action))
            .collect(),
    }))
}

#[derive(serde::Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OngoingGameStatus {
    pub id: i64,
    pub actions: Vec<String>,
}

#[derive(serde::Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameInfo {
    pub id: i64,
    pub white_id: String,
    pub black_id: String,
    pub is_rated: bool,
    pub game_settings: GameSettingsInfo,
}

impl GameInfo {
    pub fn from_ongoing_game_view(view: &GameMetadataView) -> Self {
        GameInfo {
            id: view.id.0,
            white_id: view.white_id.to_string(),
            black_id: view.black_id.to_string(),
            is_rated: view.is_rated,
            game_settings: GameSettingsInfo::from_game_settings(&view.settings),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
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

    pub fn to_game_settings(&self) -> TakGameSettings {
        TakGameSettings {
            board_size: self.board_size,
            half_komi: self.half_komi,
            reserve: tak_core::TakReserve {
                pieces: self.pieces,
                capstones: self.capstones,
            },
            time_control: tak_core::TakTimeControl {
                contingent: std::time::Duration::from_millis(self.contingent_ms),
                increment: std::time::Duration::from_millis(self.increment_ms),
                extra: self.extra.as_ref().map(|extra| {
                    (
                        extra.on_move,
                        std::time::Duration::from_millis(extra.extra_ms),
                    )
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
