use axum::{
    Json,
    extract::{Path, State},
};
use tak_server_api_contract::game::JsonGameSettings;
use tak_server_app::{
    domain::{
        TournamentId,
        tournament::{TournamentFormat, TournamentStatus},
    },
    services::player_resolver::ResolveError,
    workflow::tournament::{
        TournamentDetailView, TournamentMetadataView, TournamentView,
        register::TournamentRegistrationError,
    },
};

use crate::{AppState, ServiceError, auth::Auth, matches::JsonMatchSettings};

pub fn register_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", axum::routing::get(get_tournaments))
        .route("/", axum::routing::post(create_tournament))
        .route("/{tournament_id}", axum::routing::get(get_tournament))
        .route(
            "/{tournament_id}/start",
            axum::routing::post(start_tournament),
        )
        .route(
            "/{tournament_id}/next-round",
            axum::routing::post(start_next_round_of_tournament),
        )
        .route(
            "/{tournament_id}/finish",
            axum::routing::post(finish_tournament),
        )
        .route(
            "/{tournament_id}/players",
            axum::routing::post(register_player_to_tournament),
        )
        .route(
            "/{tournament_id}/players",
            axum::routing::delete(unregister_player_from_tournament),
        )
}

pub async fn get_tournaments(
    State(app): State<AppState>,
) -> Result<Json<Vec<JsonTournament>>, ServiceError> {
    match app.app.get_tournaments_use_case.get_tournaments().await {
        Ok(tournaments) => Ok(Json(
            tournaments
                .into_iter()
                .map(|t| JsonTournament::from(&t))
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
) -> Result<Json<JsonTournamentDetail>, ServiceError> {
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
    Ok(Json(JsonTournamentDetail::from(&tournament)))
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
        .map_err(|e| match e {
            TournamentRegistrationError::TournamentNotFound => {
                ServiceError::NotFound("Tournament not found".to_string())
            }
            TournamentRegistrationError::TournamentNotUpcoming => ServiceError::BadRequest(
                "Tournament is not upcoming, registration is closed".to_string(),
            ),
            TournamentRegistrationError::StorageError => {
                ServiceError::Internal("Failed to register player in tournament".to_string())
            }
        })
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
        .map_err(|e| match e {
            TournamentRegistrationError::TournamentNotFound => {
                ServiceError::NotFound("Tournament not found".to_string())
            }
            TournamentRegistrationError::TournamentNotUpcoming => ServiceError::BadRequest(
                "Tournament is not upcoming, registration is closed".to_string(),
            ),
            TournamentRegistrationError::StorageError => {
                ServiceError::Internal("Failed to unregister player from tournament".to_string())
            }
        })
}

pub async fn create_tournament(
    auth: Auth,
    State(app): State<AppState>,
    Json(payload): Json<CreateTournamentRequest>,
) -> Result<(), ServiceError> {
    if !auth.account.is_admin() {
        return Err(ServiceError::Unauthorized(
            "Only admins can create tournaments".to_string(),
        ));
    }
    match app
        .app
        .host_tournament_use_case
        .create_tournament(
            payload.name,
            match payload.tournament_format {
                JsonTournamentType::Swiss { rounds } => TournamentFormat::Swiss {
                    rounds: rounds as usize,
                },
                JsonTournamentType::RoundRobin => TournamentFormat::RoundRobin,
                JsonTournamentType::GroupRoundRobin { group_size } => {
                    TournamentFormat::GroupRoundRobin {
                        group_size: group_size as usize,
                    }
                }
            },
            payload.match_settings.to_match_settings(),
        )
        .await
    {
        Ok(_) => {}
        Err(_) => {
            return Err(ServiceError::Internal(
                "Failed to create tournament".to_string(),
            ));
        }
    };
    Ok(())
}

pub async fn start_tournament(
    auth: Auth,
    State(app): State<AppState>,
    Path(tournament_id): Path<String>,
) -> Result<(), ServiceError> {
    if !auth.account.is_admin() {
        return Err(ServiceError::Unauthorized(
            "Only admins can start tournaments".to_string(),
        ));
    }
    let tournament_id = TournamentId(
        tournament_id
            .parse()
            .map_err(|_| ServiceError::BadRequest("Invalid tournament ID".to_string()))?,
    );
    match app
        .app
        .host_tournament_use_case
        .begin_tournament(tournament_id)
        .await
    {
        Ok(_) => {}
        Err(_) => {
            return Err(ServiceError::Internal(
                "Failed to start tournament".to_string(),
            ));
        }
    };
    Ok(())
}

pub async fn finish_tournament(
    auth: Auth,
    State(app): State<AppState>,
    Path(tournament_id): Path<String>,
) -> Result<(), ServiceError> {
    if !auth.account.is_admin() {
        return Err(ServiceError::Unauthorized(
            "Only admins can finish tournaments".to_string(),
        ));
    }
    let tournament_id = TournamentId(
        tournament_id
            .parse()
            .map_err(|_| ServiceError::BadRequest("Invalid tournament ID".to_string()))?,
    );
    match app
        .app
        .host_tournament_use_case
        .finish_tournament(tournament_id)
        .await
    {
        Ok(_) => {}
        Err(_) => {
            return Err(ServiceError::Internal(
                "Failed to finish tournament".to_string(),
            ));
        }
    };
    Ok(())
}

pub async fn start_next_round_of_tournament(
    auth: Auth,
    State(app): State<AppState>,
    Path(tournament_id): Path<String>,
) -> Result<(), ServiceError> {
    if !auth.account.is_admin() {
        return Err(ServiceError::Unauthorized(
            "Only admins can start next round of tournaments".to_string(),
        ));
    }
    let tournament_id = TournamentId(
        tournament_id
            .parse()
            .map_err(|_| ServiceError::BadRequest("Invalid tournament ID".to_string()))?,
    );
    match app
        .app
        .host_tournament_use_case
        .start_next_round(tournament_id)
        .await
    {
        Ok(_) => {}
        Err(_) => {
            return Err(ServiceError::Internal(
                "Failed to start next round of tournament".to_string(),
            ));
        }
    };
    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTournamentRequest {
    pub name: String,
    pub tournament_format: JsonTournamentType,
    pub match_settings: JsonMatchSettings,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonTournamentDetail {
    #[serde(flatten)]
    pub tournament: JsonTournament,
    pub players: Vec<JsonTournamentPlayer>,
    pub rounds: Vec<JsonTournamentRound>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonTournamentRound {
    pub matches: Vec<String>,
    pub byes: Vec<String>,
}

impl JsonTournamentDetail {
    pub fn from(tournament: &TournamentDetailView) -> Self {
        Self {
            tournament: JsonTournament::from(&tournament.tournament),
            players: tournament
                .player_scores
                .iter()
                .map(|(player_id, score)| JsonTournamentPlayer {
                    id: player_id.to_string(),
                    score: *score,
                })
                .collect(),
            rounds: tournament
                .rounds
                .iter()
                .map(|round| JsonTournamentRound {
                    matches: round.matches.iter().map(|m| m.0.to_string()).collect(),
                    byes: round.byes.iter().map(|p| p.0.to_string()).collect(),
                })
                .collect(),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonTournamentPlayer {
    pub id: String,
    pub score: u32,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonTournamentMetadata {
    pub id: String,
    pub name: String,
    pub match_settings: JsonGameSettings,
    pub tournament_format: JsonTournamentType,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonTournament {
    pub metadata: JsonTournamentMetadata,
    pub status: JsonTournamentStatus,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum JsonTournamentStatus {
    Upcoming { registration_open: bool },
    Ongoing,
    Completed,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum JsonTournamentType {
    Swiss { rounds: u32 },
    RoundRobin,
    GroupRoundRobin { group_size: u32 },
}

impl JsonTournamentMetadata {
    pub fn from(metadata: &TournamentMetadataView) -> Self {
        Self {
            id: metadata.tournament_id.0.to_string(),
            name: metadata.name.to_string(),
            match_settings: JsonGameSettings::from_game_settings(
                &metadata.match_settings.game_settings,
            ),
            tournament_format: match metadata.tournament_format {
                TournamentFormat::Swiss { rounds } => JsonTournamentType::Swiss {
                    rounds: rounds as u32,
                },
                TournamentFormat::RoundRobin => JsonTournamentType::RoundRobin,
                TournamentFormat::GroupRoundRobin { group_size } => {
                    JsonTournamentType::GroupRoundRobin {
                        group_size: group_size as u32,
                    }
                }
            },
        }
    }
}

impl JsonTournament {
    pub fn from(tournament: &TournamentView) -> Self {
        Self {
            metadata: JsonTournamentMetadata::from(&tournament.metadata),
            status: JsonTournamentStatus::from(&tournament.status),
        }
    }
}

impl JsonTournamentStatus {
    pub fn from(status: &TournamentStatus) -> Self {
        match status {
            TournamentStatus::Upcoming { registration_open } => JsonTournamentStatus::Upcoming {
                registration_open: *registration_open,
            },
            TournamentStatus::Ongoing => JsonTournamentStatus::Ongoing,
            TournamentStatus::Completed => JsonTournamentStatus::Completed,
        }
    }
}
