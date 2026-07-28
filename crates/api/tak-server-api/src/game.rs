use std::time::{Duration, Instant};

use axum::{
    Json,
    extract::{Path, State},
    routing::{get, post},
};
use tak_core::ptn::{action_to_ptn, game_result_to_string};
use tak_server_api_contract::game::{
    ForPlayer, GameStatusType, JsonEndedGameInfo, JsonGameMetadata, JsonGameRatingInfo,
    JsonGameRequest, JsonGameRequestType, JsonGameRequests, JsonGameSettings, JsonGameStatus,
    JsonPlayerSnapshot,
};
use tak_server_app::{
    domain::{
        GameId,
        game::request::{GameRequest, GameRequestType},
    },
    services::player_resolver::ResolveError,
    workflow::{
        gameplay::{
            GameMetadataView,
            do_action::{ActionResult, HandleRequestError, PlayerActionError},
        },
        history::{GameRecordView, query::GameQueryError},
    },
};

use crate::{AppState, ServiceError, auth::Auth};

pub fn register_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", get(get_games))
        .route("/{game_id}", get(get_game_status))
        .route("/{game_id}/resign", post(resign_game))
        .route("/{game_id}/request", post(set_request))
        .route("/{game_id}/request/accept", post(accept_request))
}

pub async fn get_games(State(app): State<AppState>) -> Json<Vec<JsonGameMetadata>> {
    let games = app.app.game_list_ongoing_use_case.list_games();
    Json(
        games
            .into_iter()
            .map(|game| from_metadata_view(game.id, &game.metadata))
            .collect(),
    )
}

pub async fn get_game_status(
    State(app): State<AppState>,
    Path(game_id): Path<i64>,
) -> Result<Json<JsonGameStatus>, ServiceError> {
    let game_id = GameId(game_id);
    let game = app.app.game_get_ongoing_use_case.get_game(game_id);

    if let Some(ongoing_game) = game {
        let white_requests = JsonGameRequests {
            draw_offered: ongoing_game.white_requests.draw_offered,
            undo_requested: ongoing_game.white_requests.undo_requested,
            more_time_offered: ongoing_game
                .white_requests
                .more_time_offered
                .map(|d| d.as_millis() as u64),
        };
        let black_requests = JsonGameRequests {
            draw_offered: ongoing_game.black_requests.draw_offered,
            undo_requested: ongoing_game.black_requests.undo_requested,
            more_time_offered: ongoing_game
                .black_requests
                .more_time_offered
                .map(|d| d.as_millis() as u64),
        };

        let time_info = ongoing_game.game.get_time_info(Instant::now());
        return Ok(Json(JsonGameStatus {
            id: ongoing_game.id.to_string(),
            match_id: ongoing_game.metadata.match_id.map(|id| id.to_string()),
            player_ids: ForPlayer {
                white: ongoing_game.metadata.white_id.to_string(),
                black: ongoing_game.metadata.black_id.to_string(),
            },
            is_rated: ongoing_game.metadata.is_rated,
            game_settings: JsonGameSettings::from_game_settings(&ongoing_game.metadata.settings),
            actions: ongoing_game
                .game
                .action_history()
                .iter()
                .map(|a| action_to_ptn(&a))
                .collect(),
            status: GameStatusType::Ongoing {
                white_requests,
                black_requests,
            },
            remaining_ms: ForPlayer {
                white: time_info.white_remaining.as_millis() as u64,
                black: time_info.black_remaining.as_millis() as u64,
            },
        }));
    }
    match app.app.game_history_query_use_case.get_game(game_id).await {
        Ok(Some(ended_game)) => {
            let status = if let Some(result) = &ended_game.result {
                GameStatusType::Ended {
                    result: game_result_to_string(&result),
                }
            } else {
                tracing::warn!(
                    "Game with id {} has no result even though it's ended.",
                    game_id
                );
                return Err(ServiceError::NotFound(format!(
                    "Game with id {} not found",
                    game_id
                )));
            };

            let time_info = ended_game.reconstruct_time_info();
            Ok(Json(JsonGameStatus {
                id: game_id.to_string(),
                match_id: ended_game.metadata.match_id.map(|id| id.to_string()),
                player_ids: ForPlayer {
                    white: ended_game.metadata.white_id.to_string(),
                    black: ended_game.metadata.black_id.to_string(),
                },
                is_rated: ended_game.metadata.is_rated,
                game_settings: JsonGameSettings::from_game_settings(&ended_game.metadata.settings),
                actions: ended_game
                    .reconstruct_action_history()
                    .iter()
                    .map(|a| action_to_ptn(&a))
                    .collect(),
                status,
                remaining_ms: ForPlayer {
                    white: time_info.white_remaining.as_millis() as u64,
                    black: time_info.black_remaining.as_millis() as u64,
                },
            }))
        }
        Ok(None) => Err(ServiceError::NotFound(format!(
            "Game with id {} not found",
            game_id
        ))),
        Err(GameQueryError::RepositoryError) => Err(ServiceError::Internal(
            "Failed to retrieve game record".to_string(),
        )),
    }
}

pub async fn resign_game(
    auth: Auth,
    State(app): State<AppState>,
    Path(game_id): Path<i64>,
) -> Result<(), ServiceError> {
    let game_id = GameId(game_id);
    let player_id = app
        .app
        .player_resolver_service
        .resolve_player_id_by_account_id(&auth.account.account_id)
        .await
        .map_err(|ResolveError::Internal| {
            ServiceError::Internal(format!(
                "Failed to resolve player id for account {}",
                auth.account.account_id
            ))
        })?;
    app.app
        .game_do_action_use_case
        .resign(game_id, player_id)
        .await
        .map_err(|e| match e {
            PlayerActionError::GameNotFound => {
                ServiceError::NotFound(format!("Game with id {} not found", game_id))
            }
            PlayerActionError::NotAPlayerInGame => {
                ServiceError::Forbidden("You are not a player in this game".to_string())
            }
        })
}

pub async fn set_request(
    auth: Auth,
    State(app): State<AppState>,
    Path(game_id): Path<i64>,
    Json(request): Json<JsonGameRequest>,
) -> Result<(), ServiceError> {
    let game_id = GameId(game_id);
    let player_id = app
        .app
        .player_resolver_service
        .resolve_player_id_by_account_id(&auth.account.account_id)
        .await
        .map_err(|ResolveError::Internal| {
            ServiceError::Internal(format!(
                "Failed to resolve player id for account {}",
                auth.account.account_id
            ))
        })?;
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
        .set_request(game_id, player_id, request)
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
        },
    }
}

pub async fn accept_request(
    auth: Auth,
    State(app): State<AppState>,
    Path(game_id): Path<i64>,
    Json(request_type): Json<JsonGameRequestType>,
) -> Result<(), ServiceError> {
    let game_id = GameId(game_id);

    let player_id = app
        .app
        .player_resolver_service
        .resolve_player_id_by_account_id(&auth.account.account_id)
        .await
        .map_err(|ResolveError::Internal| {
            ServiceError::Internal(format!(
                "Failed to resolve player id for account {}",
                auth.account.account_id
            ))
        })?;

    let request_type = match request_type {
        JsonGameRequestType::Draw => GameRequestType::Draw,
        JsonGameRequestType::Undo => GameRequestType::Undo,
        JsonGameRequestType::MoreTime => GameRequestType::MoreTime,
    };

    let res = app
        .app
        .game_do_action_use_case
        .accept_request(game_id, player_id, request_type)
        .await;
    tracing::info!(
        %player_id,
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
        },
        ActionResult::ActionError(HandleRequestError::RequestNotFound) => Err(
            ServiceError::NotFound("No such request to accept".to_string()),
        ),
    }
}

pub fn from_metadata_view(game_id: GameId, view: &GameMetadataView) -> JsonGameMetadata {
    JsonGameMetadata {
        id: game_id.to_string(),
        date: view.date,
        player_ids: ForPlayer {
            white: view.white_id.to_string(),
            black: view.black_id.to_string(),
        },
        is_rated: view.is_rated,
        game_settings: JsonGameSettings::from_game_settings(&view.settings),
    }
}

pub fn from_game_record(record: &GameRecordView) -> JsonEndedGameInfo {
    JsonEndedGameInfo {
        metadata: from_metadata_view(record.game_id, &record.metadata),
        white: JsonPlayerSnapshot {
            username: record.white.username.clone(),
            rating: record.white.rating,
        },
        black: JsonPlayerSnapshot {
            username: record.black.username.clone(),
            rating: record.black.rating,
        },
        rating_info: record.rating_info.as_ref().map(|info| JsonGameRatingInfo {
            rating_change: ForPlayer {
                white: info.rating_change_white,
                black: info.rating_change_black,
            },
        }),
        result: record.result.as_ref().map(|r| game_result_to_string(r)),
    }
}
