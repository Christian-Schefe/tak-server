use axum::{
    Json,
    extract::{Path, State},
};
use tak_server_api_contract::{
    game::{GameSettingsInfo, JsonEndedGameInfo},
    matches::MatchReadinessStatus,
};
use tak_server_app::{
    domain::{
        MatchId,
        matches::{Match, MatchStatus},
    },
    services::player_resolver::ResolveError,
    workflow::{
        history::query::GameQueryError,
        matchmaking::{get::GetMatchError, readiness::MatchReadinessError},
    },
};

use crate::{AppState, ServiceError, auth::Auth, game::from_game_record};

pub fn register_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/{match_id}", axum::routing::get(get_match))
        .route("/{match_id}/games", axum::routing::get(get_match_games))
        .route(
            "/{match_id}/readiness",
            axum::routing::get(get_match_readiness_status),
        )
        .route(
            "/{match_id}/readiness",
            axum::routing::post(set_player_ready),
        )
        .route(
            "/{match_id}/readiness",
            axum::routing::delete(set_player_not_ready),
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
    Waiting,
    InProgress,
    Completed,
}

impl JsonMatchStatus {
    pub fn from_match_status(status: &MatchStatus) -> Self {
        match status {
            MatchStatus::Waiting => JsonMatchStatus::Waiting,
            MatchStatus::InProgress => JsonMatchStatus::InProgress,
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

pub async fn set_player_ready(
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
        .match_readiness_use_case
        .set_player_ready(match_id, player_id)
        .await
    {
        match e {
            MatchReadinessError::Internal => Err(ServiceError::BadRequest(
                "Failed to set match readiness".into(),
            )),
            MatchReadinessError::MatchNotFound => {
                Err(ServiceError::NotFound("Match not found".into()))
            }
        }
    } else {
        Ok(Json(()))
    }
}

pub async fn set_player_not_ready(
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
        .match_readiness_use_case
        .set_player_not_ready(match_id, player_id)
        .await
    {
        match e {
            MatchReadinessError::Internal => Err(ServiceError::Internal(
                "Failed to set match readiness".into(),
            )),
            MatchReadinessError::MatchNotFound => {
                Err(ServiceError::NotFound("Match not found".into()))
            }
        }
    } else {
        Ok(Json(()))
    }
}

pub async fn get_match_readiness_status(
    State(app): State<AppState>,
    Path(match_id): Path<String>,
) -> Result<Json<MatchReadinessStatus>, ServiceError> {
    let match_id = MatchId(
        match_id
            .parse::<i64>()
            .map_err(|_| ServiceError::BadRequest(format!("Invalid match ID: {}", match_id)))?,
    );
    let match_readiness_status = app
        .app
        .match_readiness_use_case
        .get_readiness_status(match_id);
    Ok(Json(MatchReadinessStatus {
        ready_player: match_readiness_status
            .player_ready
            .map(|player_id| player_id.to_string()),
    }))
}

pub async fn get_match_games(
    State(app): State<AppState>,
    Path(match_id): Path<String>,
) -> Result<Json<Vec<JsonEndedGameInfo>>, ServiceError> {
    let match_id = MatchId(
        match_id
            .parse::<i64>()
            .map_err(|_| ServiceError::BadRequest(format!("Invalid match ID: {}", match_id)))?,
    );
    match app
        .app
        .game_history_query_use_case
        .get_games_of_match(match_id)
        .await
    {
        Ok(games) => Ok(Json(games.iter().map(|x| from_game_record(x)).collect())),
        Err(GameQueryError::RepositoryError) => Err(ServiceError::Internal(format!(
            "Failed to retrieve games of match {}",
            match_id
        ))),
    }
}
