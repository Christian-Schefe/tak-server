use std::time::Instant;

use axum::{
    Json,
    extract::{Path, State},
};
use tak_core::{
    TakPlayer,
    ptn::{action_to_ptn, game_result_to_string},
};
use tak_server_api_contract::game::{
    ForPlayer, GameRequest, GameSettingsInfo, GameStatus, GameStatusType, JsonEndedGameInfo,
    JsonGameMetadata, JsonGameRatingInfo, JsonGameRequestType, JsonPlayerSnapshot, RequestResponse,
};
use tak_server_app::{
    domain::{
        GameId,
        game::request::{GameRequestId, GameRequestType},
    },
    services::player_resolver::ResolveError,
    workflow::{
        gameplay::{
            GameMetadataView,
            do_action::{ActionResult, AddRequestError, HandleRequestError, PlayerActionError},
        },
        history::{GameRecordView, query::GameQueryError},
    },
};

use crate::{AppState, ServiceError, auth::Auth};

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
) -> Result<Json<GameStatus>, ServiceError> {
    let game_id = GameId(game_id);
    let game = app.app.game_get_ongoing_use_case.get_game(game_id);

    if let Some(ongoing_game) = game {
        let mut requests = Vec::new();
        for request in ongoing_game.requests.into_iter() {
            let req = GameRequest {
                id: request.id.0,
                request_type: match request.request_type {
                    GameRequestType::Draw => JsonGameRequestType::Draw,
                    GameRequestType::Undo => JsonGameRequestType::Undo,
                    GameRequestType::MoreTime(_) => continue, // currently not exposed
                },
                from_player_id: match request.player {
                    TakPlayer::White => ongoing_game.metadata.white_id.to_string(),
                    TakPlayer::Black => ongoing_game.metadata.black_id.to_string(),
                },
            };
            requests.push(req);
        }
        let time_info = ongoing_game.game.get_time_info(Instant::now());
        return Ok(Json(GameStatus {
            id: ongoing_game.id.0,
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
            status: GameStatusType::Ongoing { requests },
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
                GameStatusType::Aborted // means game ended was never saved after it ended (e.g. due to server restart killing ongoing games)
            };

            let time_info = ended_game.reconstruct_time_info();
            Ok(Json(GameStatus {
                id: game_id.0,
                player_ids: ForPlayer {
                    white: ended_game.metadata.white_id.to_string(),
                    black: ended_game.metadata.black_id.to_string(),
                },
                is_rated: ended_game.metadata.is_rated,
                game_settings: GameSettingsInfo::from_game_settings(&ended_game.metadata.settings),
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
            game_id.0
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
                ServiceError::NotFound(format!("Game with id {} not found", game_id.0))
            }
            PlayerActionError::NotAPlayerInGame => {
                ServiceError::Forbidden("You are not a player in this game".to_string())
            }
        })
}

async fn add_request(
    auth: Auth,
    app: &AppState,
    game_id: GameId,
    request_type: GameRequestType,
) -> Result<(), ServiceError> {
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
    match app
        .app
        .game_do_action_use_case
        .add_request(game_id, player_id, request_type)
        .await
    {
        ActionResult::Success => Ok(()),
        ActionResult::NotPossible(e) => match e {
            PlayerActionError::GameNotFound => Err(ServiceError::NotFound(format!(
                "Game with id {} not found",
                game_id.0
            ))),
            PlayerActionError::NotAPlayerInGame => Err(ServiceError::Forbidden(
                "You are not a player in this game".to_string(),
            )),
        },
        ActionResult::ActionError(AddRequestError::AlreadyRequested) => Err(
            ServiceError::Forbidden("You have already made this request".to_string()),
        ),
    }
}

async fn retract_request_helper(
    auth: Auth,
    app: &AppState,
    game_id: GameId,
    request_id: GameRequestId,
) -> Result<(), ServiceError> {
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
    match app
        .app
        .game_do_action_use_case
        .retract_request(game_id, player_id, request_id)
        .await
    {
        ActionResult::Success => Ok(()),
        ActionResult::NotPossible(e) => match e {
            PlayerActionError::GameNotFound => Err(ServiceError::NotFound(format!(
                "Game with id {} not found",
                game_id.0
            ))),
            PlayerActionError::NotAPlayerInGame => Err(ServiceError::Forbidden(
                "You are not a player in this game".to_string(),
            )),
        },
        ActionResult::ActionError(HandleRequestError::RequestNotFound) => Err(
            ServiceError::NotFound("No such request to retract".to_string()),
        ),
    }
}

async fn reject_request(
    auth: Auth,
    app: &AppState,
    game_id: GameId,
    request_id: GameRequestId,
) -> Result<(), ServiceError> {
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
    match app
        .app
        .game_do_action_use_case
        .reject_request(game_id, player_id, request_id)
        .await
    {
        ActionResult::Success => Ok(()),
        ActionResult::NotPossible(e) => match e {
            PlayerActionError::GameNotFound => Err(ServiceError::NotFound(format!(
                "Game with id {} not found",
                game_id.0
            ))),
            PlayerActionError::NotAPlayerInGame => Err(ServiceError::Forbidden(
                "You are not a player in this game".to_string(),
            )),
        },
        ActionResult::ActionError(HandleRequestError::RequestNotFound) => Err(
            ServiceError::NotFound("No such request to reject".to_string()),
        ),
    }
}

async fn accept_request(
    auth: Auth,
    app: &AppState,
    game_id: GameId,
    request_id: GameRequestId,
) -> Result<(), ServiceError> {
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
    let Some(request) = app
        .app
        .game_do_action_use_case
        .get_request(game_id, request_id)
    else {
        return Err(ServiceError::NotFound(
            "No such request to accept".to_string(),
        ));
    };
    let res = match request.request_type {
        GameRequestType::Draw => {
            app.app
                .game_do_action_use_case
                .accept_draw_request(game_id, player_id, request_id)
                .await
        }
        GameRequestType::Undo => {
            app.app
                .game_do_action_use_case
                .accept_undo_request(game_id, player_id, request_id)
                .await
        }
        GameRequestType::MoreTime(_) => {
            return Err(ServiceError::NotPossible(
                "Accepting more time requests is not supported".to_string(),
            ));
        }
    };
    tracing::info!(
        %player_id,
        ?request_id,
        %game_id,
        "ACCEPT Player is accepting request in game",
    );
    match res {
        ActionResult::Success => Ok(()),
        ActionResult::NotPossible(e) => match e {
            PlayerActionError::GameNotFound => Err(ServiceError::NotFound(format!(
                "Game with id {} not found",
                game_id.0
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

pub async fn add_draw_request(
    auth: Auth,
    State(app): State<AppState>,
    Path(game_id): Path<i64>,
) -> Result<(), ServiceError> {
    let game_id = GameId(game_id);
    add_request(auth, &app, game_id, GameRequestType::Draw).await
}

pub async fn add_undo_request(
    auth: Auth,
    State(app): State<AppState>,
    Path(game_id): Path<i64>,
) -> Result<(), ServiceError> {
    let game_id = GameId(game_id);
    add_request(auth, &app, game_id, GameRequestType::Undo).await
}

pub async fn retract_request(
    auth: Auth,
    State(app): State<AppState>,
    Path((game_id, request_id)): Path<(i64, u64)>,
) -> Result<(), ServiceError> {
    let game_id = GameId(game_id);
    let request_id = GameRequestId(request_id);
    retract_request_helper(auth, &app, game_id, request_id).await
}

pub async fn respond_to_request(
    auth: Auth,
    State(app): State<AppState>,
    Path((game_id, request_id)): Path<(i64, u64)>,
    Json(response): Json<RequestResponse>,
) -> Result<(), ServiceError> {
    let game_id = GameId(game_id);
    let request_id = GameRequestId(request_id);
    if response.accept {
        accept_request(auth, &app, game_id, request_id).await
    } else {
        reject_request(auth, &app, game_id, request_id).await
    }
}

pub fn from_metadata_view(game_id: GameId, view: &GameMetadataView) -> JsonGameMetadata {
    JsonGameMetadata {
        id: game_id.0,
        date: view.date,
        player_ids: ForPlayer {
            white: view.white_id.to_string(),
            black: view.black_id.to_string(),
        },
        is_rated: view.is_rated,
        game_settings: GameSettingsInfo::from_game_settings(&view.settings),
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
