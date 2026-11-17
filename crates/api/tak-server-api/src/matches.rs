use axum::{
    Json,
    extract::{Path, State},
};
use tak_domain::{
    MatchId,
    matches::{Match, MatchMode, MatchSettings, MatchStatus},
};
use tak_server_api_contract::{
    game::{GameRecordResponse, GameSettingsResponse},
    matches::MatchReadinessStatusResponse,
};
use tak_server_app::application::{
    gameplay::GameQueryError,
    matchmaking::{GetMatchError, MatchReadinessError},
};

use crate::{AppState, ServiceError, auth::Auth, game::from_game};

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

async fn get_match(
    State(app): State<AppState>,
    Path(match_id): Path<String>,
) -> Result<Json<MatchResponse>, ServiceError> {
    let match_id = MatchId(
        match_id
            .parse::<i64>()
            .map_err(|_| ServiceError::BadRequest(format!("Invalid match ID: {}", match_id)))?,
    );
    match app.app.matchmaking_use_case.get_match(match_id).await {
        Ok(m) => Ok(Json(MatchResponse::from_match(match_id, m))),
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
struct MatchResponse {
    id: String,
    player1: MatchPlayerResponse,
    player2: MatchPlayerResponse,
    settings: MatchSettingsResponse,
    status: MatchStatusResponse,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchSettingsResponse {
    game_settings: GameSettingsResponse,
    match_mode: MatchModeResponse,
    is_rated: bool,
}

impl MatchSettingsResponse {
    pub fn from_match_settings(settings: &MatchSettings) -> Self {
        MatchSettingsResponse {
            game_settings: GameSettingsResponse::from_game_settings(&settings.game_settings),
            match_mode: MatchModeResponse::from_match_mode(&settings.match_mode),
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
pub enum MatchModeResponse {
    Unlimited,
    FixedGames { games: u32 },
    FirstTo { score: u32 },
}

impl MatchModeResponse {
    pub fn from_match_mode(mode: &MatchMode) -> Self {
        match mode {
            MatchMode::Unlimited => MatchModeResponse::Unlimited,
            MatchMode::FixedGames(games) => MatchModeResponse::FixedGames { games: *games },
            MatchMode::FirstTo(score) => MatchModeResponse::FirstTo { score: *score },
        }
    }

    pub fn to_match_mode(&self) -> MatchMode {
        match self {
            MatchModeResponse::Unlimited => MatchMode::Unlimited,
            MatchModeResponse::FixedGames { games } => MatchMode::FixedGames(*games),
            MatchModeResponse::FirstTo { score } => MatchMode::FirstTo(*score),
        }
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MatchStatusResponse {
    Waiting,
    Ongoing,
    Completed,
}

impl MatchStatusResponse {
    pub fn from_match_status(status: &MatchStatus) -> Self {
        match status {
            MatchStatus::Waiting => MatchStatusResponse::Waiting,
            MatchStatus::Ongoing => MatchStatusResponse::Ongoing,
            MatchStatus::Completed => MatchStatusResponse::Completed,
        }
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchPlayerResponse {
    pub user_id: String,
    pub score: u32,
}

impl MatchResponse {
    pub fn from_match(id: MatchId, m: Match) -> Self {
        Self {
            id: id.to_string(),
            player1: MatchPlayerResponse {
                user_id: m.player1.user_id.to_string(),
                score: m.player1.score,
            },
            player2: MatchPlayerResponse {
                user_id: m.player2.user_id.to_string(),
                score: m.player2.score,
            },
            settings: MatchSettingsResponse::from_match_settings(&m.settings),
            status: MatchStatusResponse::from_match_status(&m.status),
        }
    }
}

async fn set_player_ready(
    auth: Auth,
    State(app): State<AppState>,
    Path(match_id): Path<String>,
) -> Result<Json<()>, ServiceError> {
    let match_id = MatchId(
        match_id
            .parse::<i64>()
            .map_err(|_| ServiceError::BadRequest(format!("Invalid match ID: {}", match_id)))?,
    );
    if let Err(e) = app
        .app
        .matchmaking_use_case
        .set_player_ready(match_id, auth.user_id)
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

async fn set_player_not_ready(
    auth: Auth,
    State(app): State<AppState>,
    Path(match_id): Path<String>,
) -> Result<Json<()>, ServiceError> {
    let match_id = MatchId(
        match_id
            .parse::<i64>()
            .map_err(|_| ServiceError::BadRequest(format!("Invalid match ID: {}", match_id)))?,
    );
    if let Err(e) = app
        .app
        .matchmaking_use_case
        .set_player_not_ready(match_id, auth.user_id)
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

async fn get_match_readiness_status(
    State(app): State<AppState>,
    Path(match_id): Path<String>,
) -> Result<Json<MatchReadinessStatusResponse>, ServiceError> {
    let match_id = MatchId(
        match_id
            .parse::<i64>()
            .map_err(|_| ServiceError::BadRequest(format!("Invalid match ID: {}", match_id)))?,
    );
    let match_readiness_status = app.app.matchmaking_use_case.get_readiness_status(match_id);
    Ok(Json(MatchReadinessStatusResponse {
        ready_player: match_readiness_status
            .player_ready
            .map(|user_id| user_id.to_string()),
    }))
}

async fn get_match_games(
    State(app): State<AppState>,
    Path(match_id): Path<String>,
) -> Result<Json<Vec<GameRecordResponse>>, ServiceError> {
    let match_id = MatchId(
        match_id
            .parse::<i64>()
            .map_err(|_| ServiceError::BadRequest(format!("Invalid match ID: {}", match_id)))?,
    );
    match app.app.gameplay_use_case.get_games_of_match(match_id).await {
        Ok(games) => Ok(Json(
            games
                .into_iter()
                .map(|(game_id, game)| from_game(game_id, &game))
                .collect(),
        )),
        Err(GameQueryError::RepositoryError) => Err(ServiceError::Internal(format!(
            "Failed to retrieve games of match {}",
            match_id
        ))),
    }
}
