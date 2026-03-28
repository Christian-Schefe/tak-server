use axum::{
    Json,
    extract::{Path, State},
};
use tak_server_api_contract::game::GameSettingsInfo;
use tak_server_app::{
    domain::{TournamentId, tournament::TournamentType},
    services::player_resolver::ResolveError,
    workflow::tournament::TournamentMetadataView,
};

use crate::{AppState, ServiceError, auth::Auth};

pub fn register_routes(router: axum::Router<AppState>) -> axum::Router<AppState> {
    router
        .route("/tournaments", axum::routing::get(get_tournaments))
        .route(
            "/tournaments/{tournament_id}",
            axum::routing::get(get_tournament),
        )
        .route(
            "/tournaments/{tournament_id}/players",
            axum::routing::post(register_player_to_tournament),
        )
        .route(
            "/tournaments/{tournament_id}/players",
            axum::routing::delete(unregister_player_from_tournament),
        )
}

pub async fn get_tournaments(
    State(app): State<AppState>,
) -> Result<Json<Vec<JsonTournamentMetadata>>, ServiceError> {
    match app
        .app
        .get_tournaments_use_case
        .get_tournaments()
        .await
    {
        Ok(tournaments) => Ok(Json(
            tournaments
                .into_iter()
                .map(|t| JsonTournamentMetadata::from(&t.metadata))
                .collect(),
        )),
        Err(_) => Err(ServiceError::Internal(
            "Failed to retrieve tournaments".to_string(),
        )),
    }
}

pub async fn get_tournament(
    State(app): State<AppState>,
    Path(tournament_id): Path<String>,
) -> Result<Json<JsonTournamentMetadata>, ServiceError> {
    let tournament_id = TournamentId(
        tournament_id
            .parse()
            .map_err(|_| ServiceError::BadRequest("Invalid tournament ID".to_string()))?,
    );
    let tournament = match app
        .app
        .get_tournaments_use_case
        .get_tournament(tournament_id)
        .await
    {
        Ok(Some(t)) => t,
        Ok(None) => return Err(ServiceError::NotFound("Tournament not found".to_string())),
        Err(_) => {
            return Err(ServiceError::Internal(
                "Failed to retrieve tournament".to_string(),
            ));
        }
    };
    Ok(Json(JsonTournamentMetadata::from(&tournament.metadata)))
}

pub async fn register_player_to_tournament(
    auth: Auth,
    State(app): State<AppState>,
    Path(tournament_id): Path<String>,
) -> Result<(), ServiceError> {
    let tournament_id = TournamentId(
        tournament_id
            .parse()
            .map_err(|_| ServiceError::BadRequest("Invalid tournament ID".to_string()))?,
    );
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
        .tournament_player_registration_use_case
        .register_player_in_tournament(tournament_id, player_id)
        .await
        .map_err(|_| ServiceError::Internal("Failed to register player in tournament".to_string()))
}

pub async fn unregister_player_from_tournament(
    auth: Auth,
    State(app): State<AppState>,
    Path(tournament_id): Path<String>,
) -> Result<(), ServiceError> {
    let tournament_id = TournamentId(
        tournament_id
            .parse()
            .map_err(|_| ServiceError::BadRequest("Invalid tournament ID".to_string()))?,
    );
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
        .tournament_player_registration_use_case
        .unregister_player_from_tournament(tournament_id, player_id)
        .await
        .map_err(|_| {
            ServiceError::Internal("Failed to unregister player from tournament".to_string())
        })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonTournamentMetadata {
    pub id: i64,
    pub name: String,
    pub match_settings: GameSettingsInfo,
    pub tournament_type: JsonTournamentType,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum JsonTournamentType {
    Swiss,
    RoundRobin,
}

impl JsonTournamentMetadata {
    pub fn from(tournament: &TournamentMetadataView) -> Self {
        Self {
            id: tournament.tournament_id.0,
            name: tournament.name.to_string(),
            match_settings: GameSettingsInfo::from_game_settings(&tournament.match_settings),
            tournament_type: match tournament.tournament_type {
                TournamentType::Swiss => JsonTournamentType::Swiss,
                TournamentType::RoundRobin => JsonTournamentType::RoundRobin,
            },
        }
    }
}
