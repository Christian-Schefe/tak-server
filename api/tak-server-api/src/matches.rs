use axum::{
    Json,
    extract::{Path, State},
};
use tak_server_api_contract::{game::GameSettingsInfo, matches::RematchStatus};
use tak_server_app::{
    domain::{
        MatchId,
        matches::{Match, MatchStatus},
    },
    services::player_resolver::ResolveError,
    workflow::matchmaking::{get::GetMatchError, rematch::RematchError},
};

use crate::{AppState, ServiceError, auth::Auth};

pub fn register_routes(router: axum::Router<AppState>) -> axum::Router<AppState> {
    router
        .route("/matches/{match_id}", axum::routing::get(get_match))
        .route(
            "/matches/{match_id}/rematch",
            axum::routing::get(get_rematch_status),
        )
        .route(
            "/matches/{match_id}/rematch",
            axum::routing::post(request_rematch),
        )
        .route(
            "/matches/{match_id}/rematch",
            axum::routing::delete(retract_rematch_request),
        )
}

pub async fn get_match(
    State(app): State<AppState>,
    Path(match_id): Path<String>,
) -> Result<Json<JsonMatch>, ServiceError> {
    let match_id = MatchId(
        match_id
            .parse::<i64>()
            .map_err(|_| ServiceError::BadRequest(format!("Invalid match ID: {}", match_id)))?,
    );
    match app.app.match_get_use_case.get_match(match_id).await {
        Ok(m) => Ok(Json(JsonMatch::from_match(match_id, m))),
        Err(GetMatchError::NotFound) => Err(ServiceError::NotFound(format!(
            "Match not found: {}",
            match_id
        ))),
        Err(GetMatchError::InternalError) => Err(ServiceError::Internal(format!(
            "Failed to retrieve match: {}",
            match_id
        ))),
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonMatch {
    pub id: String,
    pub player1_id: String,
    pub player2_id: String,
    pub game_settings: GameSettingsInfo,
    pub half_score_player1: u32,
    pub half_score_player2: u32,
    pub status: JsonMatchStatus,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum JsonMatchStatus {
    Initial,
    InProgress,
    Completed,
}

impl JsonMatchStatus {
    pub fn from_match_status(status: &MatchStatus) -> Self {
        match status {
            MatchStatus::Initial => JsonMatchStatus::Initial,
            MatchStatus::Waiting | MatchStatus::InProgress => JsonMatchStatus::InProgress,
            MatchStatus::Completed => JsonMatchStatus::Completed,
        }
    }
}

impl JsonMatch {
    pub fn from_match(id: MatchId, m: Match) -> Self {
        Self {
            id: id.to_string(),
            player1_id: m.player1.to_string(),
            player2_id: m.player2.to_string(),
            game_settings: GameSettingsInfo::from_game_settings(&m.game_settings),
            half_score_player1: m.half_score_player1,
            half_score_player2: m.half_score_player2,
            status: JsonMatchStatus::from_match_status(&m.status),
        }
    }
}

pub async fn request_rematch(
    auth: Auth,
    State(app): State<AppState>,
    Path(match_id): Path<String>,
) -> Result<Json<()>, ServiceError> {
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
    let match_id = MatchId(
        match_id
            .parse::<i64>()
            .map_err(|_| ServiceError::BadRequest(format!("Invalid match ID: {}", match_id)))?,
    );
    if let Err(e) = app
        .app
        .match_rematch_use_case
        .request_or_accept_rematch(match_id, player_id)
        .await
    {
        match e {
            RematchError::Internal => {
                Err(ServiceError::BadRequest("Failed to accept rematch".into()))
            }
            RematchError::MatchNotFound => Err(ServiceError::NotFound("Match not found".into())),
        }
    } else {
        Ok(Json(()))
    }
}

pub async fn retract_rematch_request(
    auth: Auth,
    State(app): State<AppState>,
    Path(match_id): Path<String>,
) -> Result<Json<()>, ServiceError> {
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
    let match_id = MatchId(
        match_id
            .parse::<i64>()
            .map_err(|_| ServiceError::BadRequest(format!("Invalid match ID: {}", match_id)))?,
    );
    if let Err(e) = app
        .app
        .match_rematch_use_case
        .retract_rematch_request(match_id, player_id)
        .await
    {
        match e {
            RematchError::Internal => Err(ServiceError::Internal(
                "Failed to retract rematch request".into(),
            )),
            RematchError::MatchNotFound => Err(ServiceError::NotFound("Match not found".into())),
        }
    } else {
        Ok(Json(()))
    }
}

pub async fn get_rematch_status(
    State(app): State<AppState>,
    Path(match_id): Path<String>,
) -> Result<Json<RematchStatus>, ServiceError> {
    let match_id = MatchId(
        match_id
            .parse::<i64>()
            .map_err(|_| ServiceError::BadRequest(format!("Invalid match ID: {}", match_id)))?,
    );
    let rematch_status = app.app.match_rematch_use_case.get_rematch_status(match_id);
    Ok(Json(RematchStatus {
        rematch_requested_by: rematch_status
            .rematch_requested_by
            .map(|player_id| player_id.to_string()),
    }))
}
