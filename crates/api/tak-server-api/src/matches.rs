use axum::{
    Json,
    extract::{Path, State},
};
use tak_server_api_contract::{
    game::{JsonEndedGameInfo, JsonGameSettings},
    matches::MatchReadinessStatus,
};
use tak_server_app::{
    domain::{
        MatchId,
        matches::{Match, MatchMode, MatchSettings, MatchStatus},
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
    pub player1: JsonMatchPlayer,
    pub player2: JsonMatchPlayer,
    pub settings: JsonMatchSettings,
    pub status: JsonMatchStatus,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonMatchSettings {
    game_settings: JsonGameSettings,
    match_mode: JsonMatchMode,
    is_rated: bool,
}

impl JsonMatchSettings {
    pub fn from_match_settings(settings: &MatchSettings) -> Self {
        JsonMatchSettings {
            game_settings: JsonGameSettings::from_game_settings(&settings.game_settings),
            match_mode: JsonMatchMode::from_match_mode(&settings.match_mode),
            is_rated: settings.is_rated,
        }
    }

    pub fn to_match_settings(&self) -> MatchSettings {
        MatchSettings {
            game_settings: self.game_settings.to_game_settings(),
            match_mode: self.match_mode.to_match_mode(),
            is_rated: self.is_rated,
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
#[serde(tag = "type")]
pub enum JsonMatchMode {
    Unlimited,
    FixedGames { games: u32 },
    FirstTo { score: u32 },
}

impl JsonMatchMode {
    pub fn from_match_mode(mode: &MatchMode) -> Self {
        match mode {
            MatchMode::Unlimited => JsonMatchMode::Unlimited,
            MatchMode::FixedGames(games) => JsonMatchMode::FixedGames { games: *games },
            MatchMode::FirstTo(score) => JsonMatchMode::FirstTo { score: *score },
        }
    }

    pub fn to_match_mode(&self) -> MatchMode {
        match self {
            JsonMatchMode::Unlimited => MatchMode::Unlimited,
            JsonMatchMode::FixedGames { games } => MatchMode::FixedGames(*games),
            JsonMatchMode::FirstTo { score } => MatchMode::FirstTo(*score),
        }
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum JsonMatchStatus {
    Waiting,
    Ongoing,
    Completed,
}

impl JsonMatchStatus {
    pub fn from_match_status(status: &MatchStatus) -> Self {
        match status {
            MatchStatus::Waiting => JsonMatchStatus::Waiting,
            MatchStatus::Ongoing => JsonMatchStatus::Ongoing,
            MatchStatus::Completed => JsonMatchStatus::Completed,
        }
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonMatchPlayer {
    pub player_id: String,
    pub score: u32,
}

impl JsonMatch {
    pub fn from_match(id: MatchId, m: Match) -> Self {
        Self {
            id: id.to_string(),
            player1: JsonMatchPlayer {
                player_id: m.player1.player_id.to_string(),
                score: m.player1.score,
            },
            player2: JsonMatchPlayer {
                player_id: m.player2.player_id.to_string(),
                score: m.player2.score,
            },
            settings: JsonMatchSettings::from_match_settings(&m.settings),
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
