use std::time::Duration;

use axum::{
    Json,
    extract::{Path, State},
    routing::{get, post},
};
use tak_core::ptn::{action_to_ptn, game_result_to_string};
use tak_domain::{
    GameId,
    game::{GameEventType, GameRequest, GameRequestType},
    game_record::{GameMetadata, GameRecord},
};
use tak_server_api_contract::game::{
    ForPlayerResponse, GameMetadataResponse, GameRatingInfoResponse, GameRecordResponse,
    GameRequestResponse, GameRequestResponseType, GameRequestsResponse, GameSettingsResponse,
    GameStatusResponse, GameStatusResponseType, PlayerSnapshotResponse,
};
use tak_server_app::workflow::gameplay::{
    ActionError, HandleRequestError, PlayerActionError,
    query::{GameDetailView, GameQueryError},
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
) -> Result<Json<Vec<GameMetadataResponse>>, ServiceError> {
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
) -> Result<Json<GameStatusResponse>, ServiceError> {
    let game_id = GameId(game_id);
    match app.app.game_query_use_case.get_game(game_id).await {
        Ok(Some(detail)) => Ok(Json(status_from_game_detail(game_id, detail))),
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
        .gameplay_use_case
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
    Json(request): Json<GameRequestResponse>,
) -> Result<(), ServiceError> {
    let game_id = GameId(game_id);
    let request = match request {
        GameRequestResponse::Draw { offer } => GameRequest::Draw(offer),
        GameRequestResponse::Undo { request } => GameRequest::Undo(request),
        GameRequestResponse::MoreTime { amount_ms } => {
            GameRequest::MoreTime(amount_ms.map(|ms| Duration::from_millis(ms)))
        }
    };
    match app
        .app
        .gameplay_use_case
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
    Json(request_type): Json<GameRequestResponseType>,
) -> Result<(), ServiceError> {
    let game_id = GameId(game_id);

    let request_type = match request_type {
        GameRequestResponseType::Draw => GameRequestType::Draw,
        GameRequestResponseType::Undo => GameRequestType::Undo,
        GameRequestResponseType::MoreTime => GameRequestType::MoreTime,
    };

    let res = app
        .app
        .gameplay_use_case
        .accept_request(game_id, auth.user_id, request_type)
        .await;
    tracing::info!(
        %auth.user_id,
        %game_id,
        "ACCEPT Player is accepting request in game",
    );
    match res {
        Ok(()) => Ok(()),
        Err(ActionError::NotPossible(e)) => match e {
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
        Err(ActionError::ActionError(HandleRequestError::RequestNotFound)) => Err(
            ServiceError::NotFound("No such request to accept".to_string()),
        ),
    }
}

pub fn from_metadata(game_id: GameId, metadata: &GameMetadata) -> GameMetadataResponse {
    GameMetadataResponse {
        id: game_id.to_string(),
        date: metadata.date,
        white: PlayerSnapshotResponse {
            user_id: metadata.white.user_id.to_string(),
            username: metadata.white.username.clone(),
            rating: metadata.white.rating,
        },
        black: PlayerSnapshotResponse {
            user_id: metadata.black.user_id.to_string(),
            username: metadata.black.username.clone(),
            rating: metadata.black.rating,
        },
        is_rated: metadata.is_rated,
        game_settings: GameSettingsResponse::from_game_settings(&metadata.settings),
        match_id: metadata.match_id.map(|id| id.to_string()),
    }
}

pub fn from_game(game_id: GameId, record: &GameRecord) -> GameRecordResponse {
    let result = record.result.map(|res| game_result_to_string(res));

    let rating_info = record
        .rating_info
        .as_ref()
        .map(|info| GameRatingInfoResponse {
            rating_change: ForPlayerResponse {
                white: info.rating_change_white,
                black: info.rating_change_black,
            },
        });
    let metadata = from_metadata(game_id, &record.metadata);

    GameRecordResponse {
        metadata,
        result,
        rating_info,
    }
}

pub fn status_from_game_detail(game_id: GameId, detail: GameDetailView) -> GameStatusResponse {
    let metadata = from_metadata(game_id, &detail.record.metadata);
    let mut history = vec![];
    let mut time_info = detail
        .record
        .metadata
        .settings
        .time_settings
        .initial_time_info();
    for event in detail.events {
        match event.event_type {
            GameEventType::ActionPerformed { action, .. } => {
                history.push(action_to_ptn(&action));
            }
            GameEventType::ActionUndone { .. } => {
                history.pop();
            }
            GameEventType::GameOver { .. } => {}
        };
        time_info = event.time_info;
    }

    let json_state = match detail.record.result {
        Some(res) => GameStatusResponseType::Finished {
            result: game_result_to_string(res),
        },
        None => {
            if let Some(state) = detail.state {
                let requests = ForPlayerResponse {
                    white: GameRequestsResponse {
                        draw_offered: state.white_requests.draw_offered,
                        undo_requested: state.white_requests.undo_requested,
                        more_time_offered: state
                            .white_requests
                            .more_time_offered
                            .map(|d| d.as_millis() as u64),
                    },
                    black: GameRequestsResponse {
                        draw_offered: state.black_requests.draw_offered,
                        undo_requested: state.black_requests.undo_requested,
                        more_time_offered: state
                            .black_requests
                            .more_time_offered
                            .map(|d| d.as_millis() as u64),
                    },
                };
                GameStatusResponseType::Ongoing { requests }
            } else {
                GameStatusResponseType::Ongoing {
                    requests: ForPlayerResponse {
                        white: GameRequestsResponse {
                            draw_offered: false,
                            undo_requested: false,
                            more_time_offered: None,
                        },
                        black: GameRequestsResponse {
                            draw_offered: false,
                            undo_requested: false,
                            more_time_offered: None,
                        },
                    },
                }
            }
        }
    };

    GameStatusResponse {
        metadata,
        actions: history,
        remaining_ms: ForPlayerResponse {
            white: time_info.white_remaining.as_millis() as u64,
            black: time_info.black_remaining.as_millis() as u64,
        },
        status: json_state,
    }
}
