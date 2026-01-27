use std::time::{Duration, Instant};

use axum::{
    Json,
    extract::{Path, State},
};
use tak_core::{
    TakAsyncTimeControl, TakBaseGameSettings, TakGameSettings, TakPlayer, TakRealtimeTimeControl,
    TakReserve, TakTimeInfo, TakTimeSettings,
    ptn::{action_to_ptn, game_result_to_string},
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
        history::query::GameQueryError,
    },
};

use crate::{AppState, ServiceError, auth::Auth};

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
            status: GameStatusType::Ongoing { requests },
            time_info: JsonTimeInfo::from_tak_time_info(&time_info),
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
                status,
                time_info: JsonTimeInfo::from_tak_time_info(&time_info),
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
    log::info!(
        "ACCEPT Player {} is accepting request {:?} in game {}",
        player_id,
        request_id,
        game_id
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
            ServiceError::NotFound("No such request to reject".to_string()),
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

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RequestResponse {
    pub accept: bool,
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
    pub time_info: JsonTimeInfo,
}

#[derive(serde::Serialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub struct JsonTimeInfo {
    remaining_ms: ForPlayer<u64>,
}

impl JsonTimeInfo {
    pub fn from_tak_time_info(time_info: &TakTimeInfo) -> Self {
        JsonTimeInfo {
            remaining_ms: ForPlayer {
                white: time_info.white_remaining.as_millis() as u64,
                black: time_info.black_remaining.as_millis() as u64,
            },
        }
    }
}

#[derive(serde::Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameRequest {
    id: u64,
    from_player_id: String,
    request_type: JsonGameRequestType,
}

#[derive(serde::Serialize, Debug, Clone)]
#[serde(
    rename_all = "camelCase",
    tag = "type",
    rename_all_fields = "camelCase"
)]
pub enum JsonGameRequestType {
    Draw,
    Undo,
}

#[derive(serde::Serialize, Debug, Clone)]
#[serde(
    rename_all = "camelCase",
    tag = "type",
    rename_all_fields = "camelCase"
)]
pub enum GameStatusType {
    Ongoing { requests: Vec<GameRequest> },
    Ended { result: String },
    Aborted,
}

#[derive(serde::Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ForPlayer<R> {
    pub white: R,
    pub black: R,
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
    pub time_settings: JsonTimeSettings,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(
    rename_all = "camelCase",
    tag = "type",
    rename_all_fields = "camelCase"
)]
pub enum JsonTimeSettings {
    Realtime {
        contingent_ms: u64,
        increment_ms: u64,
        extra: Option<ExtraTime>,
    },
    Async {
        increment_ms: u64,
    },
}

impl GameSettingsInfo {
    pub fn from_game_settings(settings: &TakGameSettings) -> Self {
        GameSettingsInfo {
            board_size: settings.base.board_size,
            half_komi: settings.base.half_komi,
            pieces: settings.base.reserve.pieces,
            capstones: settings.base.reserve.capstones,
            time_settings: match &settings.time_settings {
                TakTimeSettings::Realtime(tc) => JsonTimeSettings::Realtime {
                    contingent_ms: tc.contingent.as_millis() as u64,
                    increment_ms: tc.increment.as_millis() as u64,
                    extra: tc.extra.map(|(on_move, extra_time)| ExtraTime {
                        on_move,
                        extra_ms: extra_time.as_millis() as u64,
                    }),
                },
                TakTimeSettings::Async(tc) => JsonTimeSettings::Async {
                    increment_ms: tc.contingent.as_millis() as u64,
                },
            },
        }
    }

    pub fn to_game_settings(&self) -> TakGameSettings {
        TakGameSettings {
            base: TakBaseGameSettings {
                board_size: self.board_size,
                half_komi: self.half_komi,
                reserve: TakReserve {
                    pieces: self.pieces,
                    capstones: self.capstones,
                },
            },
            time_settings: match &self.time_settings {
                JsonTimeSettings::Realtime {
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
                JsonTimeSettings::Async { increment_ms } => {
                    TakTimeSettings::Async(TakAsyncTimeControl {
                        contingent: Duration::from_millis(*increment_ms),
                    })
                }
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
