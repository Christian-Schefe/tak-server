use axum::{
    Json,
    extract::{Path, State},
};
use tak_core::ptn::{action_from_ptn, action_to_ptn};
use tak_server_app::{
    domain::{PuzzleId, puzzle::PuzzleResponse},
    workflow::puzzle::{PuzzleView, get::GetPuzzleError, solve::SolvePuzzleError},
};

use crate::{AppState, ServiceError, game::GameSettingsInfoBase};

pub fn register_routes(router: axum::Router<AppState>) -> axum::Router<AppState> {
    router
        .route("/puzzles/{puzzle_id}", axum::routing::get(get_puzzle))
        .route(
            "/puzzles/{puzzle_id}",
            axum::routing::post(try_solve_puzzle),
        )
        .route("/puzzles", axum::routing::get(get_random_puzzle))
}

pub async fn get_puzzle(
    State(app): State<AppState>,
    Path(puzzle_id): Path<String>,
) -> Result<Json<JsonPuzzle>, ServiceError> {
    let puzzle_id = PuzzleId(
        puzzle_id
            .parse()
            .map_err(|_| ServiceError::BadRequest("Invalid puzzle ID".to_string()))?,
    );
    let puzzle = app
        .app
        .get_puzzle_use_case
        .get_puzzle(puzzle_id)
        .await
        .map_err(|e| match e {
            GetPuzzleError::NotFound => ServiceError::NotFound("Puzzle not found".to_string()),
            GetPuzzleError::InternalError => {
                ServiceError::Internal("Failed to retrieve puzzle".to_string())
            }
        })?;
    Ok(Json(JsonPuzzle::from(puzzle)))
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrySolvePuzzlePayload {
    pub actions: Vec<String>,
}

pub async fn try_solve_puzzle(
    State(app): State<AppState>,
    Path(puzzle_id): Path<String>,
    Json(payload): Json<TrySolvePuzzlePayload>,
) -> Result<Json<TrySolvePuzzleResponse>, ServiceError> {
    let puzzle_id = PuzzleId(
        puzzle_id
            .parse()
            .map_err(|_| ServiceError::BadRequest("Invalid puzzle ID".to_string()))?,
    );
    let actions = payload
        .actions
        .into_iter()
        .map(|action_str| {
            action_from_ptn(&action_str).ok_or_else(|| {
                ServiceError::BadRequest(format!("Invalid action in payload: {}", action_str))
            })
        })
        .collect::<Result<Vec<_>, ServiceError>>()?;
    let response = app
        .app
        .solve_puzzle_use_case
        .attempt_solve_puzzle(puzzle_id, actions)
        .await
        .map_err(|e| match e {
            SolvePuzzleError::NotFound => ServiceError::NotFound("Puzzle not found".to_string()),
            SolvePuzzleError::InternalError => {
                ServiceError::Internal("Failed to solve puzzle".to_string())
            }
            SolvePuzzleError::InvalidInput(msg) => ServiceError::BadRequest(msg),
        })?;
    let res = match response {
        PuzzleResponse::Success => TrySolvePuzzleResponse::Correct,
        PuzzleResponse::Response(action) => TrySolvePuzzleResponse::Continue {
            action: action_to_ptn(&action),
        },
        PuzzleResponse::Failure => TrySolvePuzzleResponse::Incorrect,
    };
    Ok(Json(res))
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum TrySolvePuzzleResponse {
    Correct,
    Continue { action: String },
    Incorrect,
}

pub async fn get_random_puzzle(
    State(app): State<AppState>,
) -> Result<Json<PuzzleSelection>, ServiceError> {
    let puzzle_id = app
        .app
        .get_puzzle_use_case
        .select_random_puzzle()
        .await
        .map_err(|_| ServiceError::Internal("Failed to select random puzzle".to_string()))?;
    Ok(Json(PuzzleSelection { id: puzzle_id.0 }))
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PuzzleSelection {
    pub id: i64,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonPuzzle {
    pub id: i64,
    pub actions: Vec<String>,
    pub game_settings: GameSettingsInfoBase,
}

impl JsonPuzzle {
    pub fn from(puzzle: PuzzleView) -> Self {
        Self {
            id: puzzle.id.0,
            actions: puzzle.position.iter().map(|a| action_to_ptn(a)).collect(),
            game_settings: GameSettingsInfoBase::from_base_settings(&puzzle.game_settings),
        }
    }
}
