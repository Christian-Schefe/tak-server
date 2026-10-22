use std::time::Duration;

use axum::{
    Json,
    extract::{Path, State},
    routing::{get, post},
};
use tak_core::ptn::game_result_to_string;
use tak_domain::{
    GameId,
    game::{
        GameMetadata,
        request::{GameRequest, GameRequestType},
    },
    game_record::{GameRecord, GameRecordState},
};
use tak_server_api_contract::game::{
    ForPlayer, JsonGameMetadata, JsonGameRatingInfo, JsonGameRecord, JsonGameRecordState,
    JsonGameRequest, JsonGameRequestType, JsonGameSettings, JsonPlayerSnapshot,
};
use tak_server_app::workflow::gameplay::{
    do_action::{ActionResult, HandleRequestError, PlayerActionError},
    query::GameQueryError,
};

use crate::{AppState, ServiceError, auth::Auth};

pub fn register_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", get(get_games))
        .route("/{game_id}", get(get_game))
        .route("/{game_id}/resign", post(resign_game))
        .route("/{game_id}/request", post(set_request))
        .route("/{game_id}/request/accept", post(accept_request))
}

async fn get_games(
    State(app): State<AppState>,
) -> Result<Json<Vec<JsonGameMetadata>>, ServiceError> {
    let games = match app.app.game_query_use_case.get_ongoing_games().await {
        Ok(games) => games,
        Err(GameQueryError::RepositoryError) => {
            return Err(ServiceError::Internal("Failed to get ongoing games".into()));
        }
    };
    Ok(Json(
        games
            .into_iter()
            .map(|(game_id, game)| from_metadata(game_id, &game.metadata))
            .collect(),
    ))
}

async fn get_game(
    State(app): State<AppState>,
    Path(game_id): Path<i64>,
) -> Result<Json<JsonGameRecord>, ServiceError> {
    let game_id = GameId(game_id);
    match app.app.game_query_use_case.get_game(game_id).await {
        Ok(Some(game)) => Ok(Json(from_game(game_id, &game))),
        Ok(None) => Err(ServiceError::NotFound(format!(
            "Game with id {} not found",
            game_id
        ))),
        Err(GameQueryError::RepositoryError) => Err(ServiceError::Internal(
            "Failed to retrieve game record".to_string(),
        )),
    }
}

async fn resign_game(
    auth: Auth,
    State(app): State<AppState>,
    Path(game_id): Path<i64>,
) -> Result<(), ServiceError> {
    let game_id = GameId(game_id);
    app.app
        .game_do_action_use_case
        .resign(game_id, auth.user_id)
        .await
        .map_err(|e| match e {
            PlayerActionError::GameNotFound => {
                ServiceError::NotFound(format!("Game with id {} not found", game_id))
            }
            PlayerActionError::NotAPlayerInGame => {
                ServiceError::Forbidden("You are not a player in this game".to_string())
            }
            PlayerActionError::Internal => {
                ServiceError::Internal("Failed to resign from game".to_string())
            }
        })
}

async fn set_request(
    auth: Auth,
    State(app): State<AppState>,
    Path(game_id): Path<i64>,
    Json(request): Json<JsonGameRequest>,
) -> Result<(), ServiceError> {
    let game_id = GameId(game_id);
    let request = match request {
        JsonGameRequest::Draw { offer } => GameRequest::Draw(offer),
        JsonGameRequest::Undo { request } => GameRequest::Undo(request),
        JsonGameRequest::MoreTime { amount_ms } => {
            GameRequest::MoreTime(amount_ms.map(|ms| Duration::from_millis(ms)))
        }
    };
    match app
        .app
        .game_do_action_use_case
        .set_request(game_id, auth.user_id, request)
        .await
    {
        Ok(()) => Ok(()),
        Err(e) => match e {
            PlayerActionError::GameNotFound => Err(ServiceError::NotFound(format!(
                "Game with id {} not found",
                game_id
            ))),
            PlayerActionError::NotAPlayerInGame => Err(ServiceError::Forbidden(
                "You are not a player in this game".to_string(),
            )),
            PlayerActionError::Internal => Err(ServiceError::Internal(
                "Failed to set request for game".to_string(),
            )),
        },
    }
}

async fn accept_request(
    auth: Auth,
    State(app): State<AppState>,
    Path(game_id): Path<i64>,
    Json(request_type): Json<JsonGameRequestType>,
) -> Result<(), ServiceError> {
    let game_id = GameId(game_id);

    let request_type = match request_type {
        JsonGameRequestType::Draw => GameRequestType::Draw,
        JsonGameRequestType::Undo => GameRequestType::Undo,
        JsonGameRequestType::MoreTime => GameRequestType::MoreTime,
    };

    let res = app
        .app
        .game_do_action_use_case
        .accept_request(game_id, auth.user_id, request_type)
        .await;
    tracing::info!(
        %auth.user_id,
        %game_id,
        "ACCEPT Player is accepting request in game",
    );
    match res {
        ActionResult::Success => Ok(()),
        ActionResult::NotPossible(e) => match e {
            PlayerActionError::GameNotFound => Err(ServiceError::NotFound(format!(
                "Game with id {} not found",
                game_id
            ))),
            PlayerActionError::NotAPlayerInGame => Err(ServiceError::Forbidden(
                "You are not a player in this game".to_string(),
            )),
            PlayerActionError::Internal => Err(ServiceError::Internal(
                "Failed to accept request for game".to_string(),
            )),
        },
        ActionResult::ActionError(HandleRequestError::RequestNotFound) => Err(
            ServiceError::NotFound("No such request to accept".to_string()),
        ),
    }
}

pub fn from_metadata(game_id: GameId, metadata: &GameMetadata) -> JsonGameMetadata {
    JsonGameMetadata {
        id: game_id.to_string(),
        date: metadata.date,
        white: JsonPlayerSnapshot {
            user_id: metadata.white.user_id.to_string(),
            username: metadata.white.username.clone(),
            rating: metadata.white.rating,
        },
        black: JsonPlayerSnapshot {
            user_id: metadata.black.user_id.to_string(),
            username: metadata.black.username.clone(),
            rating: metadata.black.rating,
        },
        is_rated: metadata.is_rated,
        game_settings: JsonGameSettings::from_game_settings(&metadata.settings),
    }
}

pub fn from_game(game_id: GameId, game: &GameRecord) -> JsonGameRecord {
    JsonGameRecord {
        metadata: from_metadata(game_id, &game.metadata),
        state: match &game.state {
            GameRecordState::Ongoing => JsonGameRecordState::Ongoing,
            GameRecordState::Finished {
                history: _,
                time_info,
                rating_info,
                result,
            } => JsonGameRecordState::Finished {
                rating_info: rating_info.as_ref().map(|info| JsonGameRatingInfo {
                    rating_change: ForPlayer {
                        white: info.rating_change_white,
                        black: info.rating_change_black,
                    },
                }),
                result: game_result_to_string(&result),
                remaining_ms: ForPlayer {
                    white: time_info.white_remaining.as_millis() as u64,
                    black: time_info.black_remaining.as_millis() as u64,
                },
            },
        },
    }
}
