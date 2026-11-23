use axum::{
    Json,
    extract::{Path, State},
};
use tak_domain::{
    TournamentId,
    tournament::{TournamentFormat, TournamentStatus},
};
use tak_server_app::{
    application::tournament::{TournamentDetailView, TournamentMetadataView, TournamentView},
    errors::OneOf,
};

use crate::{AppState, ServiceError, auth::Auth, matches::MatchSettingsResponse};

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
        .route(
            "/{tournament_id}/registration",
            axum::routing::post(set_registration_open),
        )
}

async fn get_tournaments(
    State(app): State<AppState>,
) -> Result<Json<Vec<TournamentResponse>>, ServiceError> {
    let tournaments = app
        .app
        .tournament_use_case
        .get_tournaments()
        .await
        .map_err(OneOf::convert::<ServiceError>)?;
    Ok(Json(
        tournaments
            .into_iter()
            .map(|t| TournamentResponse::from(&t))
            .collect(),
    ))
}

async fn get_tournament(
    State(app): State<AppState>,
    Path(tournament_id): Path<String>,
) -> Result<Json<TournamentDetailResponse>, ServiceError> {
    let tournament_id = TournamentId(
        tournament_id
            .parse()
            .map_err(|_| ServiceError::BadRequest("Invalid tournament ID".to_string()))?,
    );
    let tournament = match app
        .app
        .tournament_use_case
        .get_tournament(tournament_id)
        .await
        .map_err(OneOf::convert::<ServiceError>)?
    {
        Some(t) => t,
        None => return Err(ServiceError::NotFound("Tournament not found".to_string())),
    };
    Ok(Json(TournamentDetailResponse::from(&tournament)))
}

async fn register_player_to_tournament(
    auth: Auth,
    State(app): State<AppState>,
    Path(tournament_id): Path<String>,
) -> Result<(), ServiceError> {
    let tournament_id = TournamentId(
        tournament_id
            .parse()
            .map_err(|_| ServiceError::BadRequest("Invalid tournament ID".to_string()))?,
    );
    app.app
        .tournament_use_case
        .register_player_in_tournament(tournament_id, auth.user_id)
        .await
        .map_err(OneOf::convert::<ServiceError>)?;
    Ok(())
}

async fn unregister_player_from_tournament(
    auth: Auth,
    State(app): State<AppState>,
    Path(tournament_id): Path<String>,
) -> Result<(), ServiceError> {
    let tournament_id = TournamentId(
        tournament_id
            .parse()
            .map_err(|_| ServiceError::BadRequest("Invalid tournament ID".to_string()))?,
    );
    app.app
        .tournament_use_case
        .unregister_player_from_tournament(tournament_id, auth.user_id)
        .await
        .map_err(OneOf::convert::<ServiceError>)?;
    Ok(())
}

async fn create_tournament(
    auth: Auth,
    State(app): State<AppState>,
    Json(payload): Json<CreateTournamentRequest>,
) -> Result<(), ServiceError> {
    let format = match payload.tournament_format {
        TournamentTypeResponse::Swiss { rounds } => TournamentFormat::Swiss {
            rounds: rounds as usize,
        },
        TournamentTypeResponse::RoundRobin => TournamentFormat::RoundRobin,
        TournamentTypeResponse::GroupRoundRobin { group_size } => {
            TournamentFormat::GroupRoundRobin {
                group_size: group_size as usize,
            }
        }
    };
    app.app
        .tournament_use_case
        .create_tournament(
            auth.user_id,
            payload.name,
            format,
            payload.match_settings.to_match_settings(),
        )
        .await
        .map_err(OneOf::convert::<ServiceError>)?;
    Ok(())
}

async fn start_tournament(
    auth: Auth,
    State(app): State<AppState>,
    Path(tournament_id): Path<String>,
) -> Result<(), ServiceError> {
    let tournament_id = TournamentId(
        tournament_id
            .parse()
            .map_err(|_| ServiceError::BadRequest("Invalid tournament ID".to_string()))?,
    );
    app.app
        .tournament_use_case
        .begin_tournament(auth.user_id, tournament_id)
        .await
        .map_err(OneOf::convert::<ServiceError>)?;
    Ok(())
}

async fn finish_tournament(
    auth: Auth,
    State(app): State<AppState>,
    Path(tournament_id): Path<String>,
) -> Result<(), ServiceError> {
    let tournament_id = TournamentId(
        tournament_id
            .parse()
            .map_err(|_| ServiceError::BadRequest("Invalid tournament ID".to_string()))?,
    );
    app.app
        .tournament_use_case
        .finish_tournament(auth.user_id, tournament_id)
        .await
        .map_err(OneOf::convert::<ServiceError>)?;
    Ok(())
}

async fn start_next_round_of_tournament(
    auth: Auth,
    State(app): State<AppState>,
    Path(tournament_id): Path<String>,
) -> Result<(), ServiceError> {
    let tournament_id = TournamentId(
        tournament_id
            .parse()
            .map_err(|_| ServiceError::BadRequest("Invalid tournament ID".to_string()))?,
    );
    app.app
        .tournament_use_case
        .start_next_round(auth.user_id, tournament_id)
        .await
        .map_err(OneOf::convert::<ServiceError>)?;
    Ok(())
}

async fn set_registration_open(
    auth: Auth,
    State(app): State<AppState>,
    Path(tournament_id): Path<String>,
    Json(payload): Json<SetRegistrationOpenRequest>,
) -> Result<(), ServiceError> {
    let tournament_id = TournamentId(
        tournament_id
            .parse()
            .map_err(|_| ServiceError::BadRequest("Invalid tournament ID".to_string()))?,
    );
    app.app
        .tournament_use_case
        .set_registration_open(auth.user_id, tournament_id, payload.registration_open)
        .await
        .map_err(OneOf::convert::<ServiceError>)?;
    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetRegistrationOpenRequest {
    registration_open: bool,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateTournamentRequest {
    name: String,
    tournament_format: TournamentTypeResponse,
    match_settings: MatchSettingsResponse,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct TournamentDetailResponse {
    #[serde(flatten)]
    tournament: TournamentResponse,
    players: Vec<TournamentPlayerResponse>,
    rounds: Vec<TournamentRoundResponse>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct TournamentRoundResponse {
    matches: Vec<String>,
    byes: Vec<String>,
}

impl TournamentDetailResponse {
    fn from(tournament: &TournamentDetailView) -> Self {
        Self {
            tournament: TournamentResponse::from(&tournament.tournament),
            players: tournament
                .player_scores
                .iter()
                .map(|(user_id, score)| TournamentPlayerResponse {
                    id: user_id.to_string(),
                    score: *score,
                })
                .collect(),
            rounds: tournament
                .rounds
                .iter()
                .map(|round| TournamentRoundResponse {
                    matches: round.matches.iter().map(|m| m.0.to_string()).collect(),
                    byes: round.byes.iter().map(|p| p.0.to_string()).collect(),
                })
                .collect(),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct TournamentPlayerResponse {
    id: String,
    score: u32,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct TournamentMetadataResponse {
    id: String,
    name: String,
    match_settings: MatchSettingsResponse,
    tournament_format: TournamentTypeResponse,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct TournamentResponse {
    metadata: TournamentMetadataResponse,
    status: TournamentStatusResponse,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum TournamentStatusResponse {
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
pub enum TournamentTypeResponse {
    Swiss { rounds: u32 },
    RoundRobin,
    GroupRoundRobin { group_size: u32 },
}

impl TournamentMetadataResponse {
    pub fn from(metadata: &TournamentMetadataView) -> Self {
        Self {
            id: metadata.tournament_id.0.to_string(),
            name: metadata.name.to_string(),
            match_settings: MatchSettingsResponse::from_match_settings(&metadata.match_settings),
            tournament_format: match metadata.tournament_format {
                TournamentFormat::Swiss { rounds } => TournamentTypeResponse::Swiss {
                    rounds: rounds as u32,
                },
                TournamentFormat::RoundRobin => TournamentTypeResponse::RoundRobin,
                TournamentFormat::GroupRoundRobin { group_size } => {
                    TournamentTypeResponse::GroupRoundRobin {
                        group_size: group_size as u32,
                    }
                }
            },
        }
    }
}

impl TournamentResponse {
    pub fn from(tournament: &TournamentView) -> Self {
        Self {
            metadata: TournamentMetadataResponse::from(&tournament.metadata),
            status: TournamentStatusResponse::from(&tournament.status),
        }
    }
}

impl TournamentStatusResponse {
    pub fn from(status: &TournamentStatus) -> Self {
        match status {
            TournamentStatus::Upcoming { registration_open } => {
                TournamentStatusResponse::Upcoming {
                    registration_open: *registration_open,
                }
            }
            TournamentStatus::Ongoing => TournamentStatusResponse::Ongoing,
            TournamentStatus::Completed => TournamentStatusResponse::Completed,
        }
    }
}
