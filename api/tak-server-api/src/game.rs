use std::time::Instant;

use axum::{
    Json,
    extract::{Path, State},
};
use tak_core::{
    TakGameSettings,
    ptn::{action_to_ptn, game_state_to_string},
};
use tak_server_app::{
    domain::GameId,
    workflow::{gameplay::GameMetadataView, history::query::GameQueryError},
};

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

pub async fn get_game_status(
    State(app): State<AppState>,
    Path(game_id): Path<i64>,
) -> Result<Json<GameStatus>, ServiceError> {
    let game_id = GameId(game_id);
    let game = app.app.game_get_ongoing_use_case.get_game(game_id);

    if let Some(ongoing_game) = game {
        let (draw_offer_white, draw_offer_black) = ongoing_game.game.draw_offers();
        let (undo_request_white, undo_request_black) = ongoing_game.game.undo_requests();
        let (white_remaining, black_remaining) =
            ongoing_game.game.get_time_remaining_both(Instant::now());
        return Ok(Json(GameStatus {
            id: ongoing_game.metadata.id.0,
            player_ids: ForPlayer {
                white: ongoing_game.metadata.white_id.to_string(),
                black: ongoing_game.metadata.black_id.to_string(),
            },
            is_rated: ongoing_game.metadata.is_rated,
            game_settings: GameSettingsInfo::from_game_settings(&ongoing_game.metadata.settings),
            actions: ongoing_game
                .game
                .action_history()
                .iter()
                .map(|a| action_to_ptn(&a))
                .collect(),
            status: GameStatusType::Ongoing {
                draw_offers: ForPlayer {
                    white: draw_offer_white,
                    black: draw_offer_black,
                },
                undo_requests: ForPlayer {
                    white: undo_request_white,
                    black: undo_request_black,
                },
            },
            remaining_ms: ForPlayer {
                white: white_remaining.as_millis() as u64,
                black: black_remaining.as_millis() as u64,
            },
        }));
    }
    match app.app.game_history_query_use_case.get_game(game_id).await {
        Ok(Some(ended_game)) => {
            let Some(result) = &ended_game.result else {
                log::warn!("Ended game {} has no result", game_id.0);
                return Err(ServiceError::NotPossible(
                    "Game finished but not processed yet".to_string(),
                ));
            };
            Ok(Json(GameStatus {
                id: game_id.0,
                player_ids: ForPlayer {
                    white: ended_game.white.player_id.to_string(),
                    black: ended_game.black.player_id.to_string(),
                },
                is_rated: ended_game.is_rated,
                game_settings: GameSettingsInfo::from_game_settings(&ended_game.settings),
                actions: ended_game
                    .reconstruct_action_history()
                    .iter()
                    .map(|a| action_to_ptn(&a))
                    .collect(),
                status: GameStatusType::Ended {
                    result: game_state_to_string(&result),
                },
                remaining_ms: ForPlayer { white: 0, black: 0 },
            }))
        }
        Ok(None) => Err(ServiceError::NotFound(format!(
            "Game with id {} not found",
            game_id.0
        ))),
        Err(GameQueryError::RepositoryError) => Err(ServiceError::Internal(
            "Failed to retrieve game record".to_string(),
        )),
    }
}

#[derive(serde::Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameStatus {
    pub id: i64,
    pub player_ids: ForPlayer<String>,
    pub is_rated: bool,
    pub game_settings: GameSettingsInfo,
    pub actions: Vec<String>,
    pub status: GameStatusType,
    pub remaining_ms: ForPlayer<u64>,
}

#[derive(serde::Serialize, Debug, Clone)]
#[serde(
    rename_all = "camelCase",
    tag = "type",
    rename_all_fields = "camelCase"
)]
pub enum GameStatusType {
    Ongoing {
        draw_offers: ForPlayer<bool>,
        undo_requests: ForPlayer<bool>,
    },
    Ended {
        result: String,
    },
}

#[derive(serde::Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ForPlayer<R> {
    white: R,
    black: R,
}

#[derive(serde::Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameInfo {
    pub id: i64,
    pub player_ids: ForPlayer<String>,
    pub is_rated: bool,
    pub game_settings: GameSettingsInfo,
}

impl GameInfo {
    pub fn from_ongoing_game_view(view: &GameMetadataView) -> Self {
        GameInfo {
            id: view.id.0,
            player_ids: ForPlayer {
                white: view.white_id.to_string(),
                black: view.black_id.to_string(),
            },
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
