use axum::{
    Json,
    extract::{Path, State},
};
use tak_server_app::{
    domain::{AccountId, PlayerId},
    workflow::player::PlayerStatsView,
};
use uuid::Uuid;

use crate::{AppState, ServiceError};

async fn get_player_info_helper(
    app: &AppState,
    player_id: PlayerId,
) -> Result<PlayerInfo, ServiceError> {
    let account = app
        .app
        .get_account_workflow
        .get_account(player_id)
        .await
        .map_err(|_| ServiceError::Internal("Failed to retrieve player account".to_string()))?;

    let rating = app
        .app
        .player_get_rating_use_case
        .get_rating(player_id)
        .await;
    let rating = match rating {
        Ok(Some(rating_view)) => Some(RatingResponse {
            rating: rating_view.rating,
            participation_rating: rating_view.participation_rating,
        }),
        Ok(None) => None,
        Err(_) => {
            return Err(ServiceError::Internal(
                "Failed to retrieve player rating".to_string(),
            ));
        }
    };

    Ok(PlayerInfo {
        id: player_id.to_string(),
        account_id: account.account_id.to_string(),
        username: account.username,
        display_name: account.display_name,
        rating,
    })
}

pub async fn get_player_info(
    State(app): State<AppState>,
    Path(player_id): Path<String>,
) -> Result<Json<PlayerInfo>, ServiceError> {
    let player_id = PlayerId(
        Uuid::parse_str(&player_id)
            .map_err(|_| ServiceError::BadRequest("Invalid player ID".to_string()))?,
    );
    get_player_info_helper(&app, player_id).await.map(Json)
}

pub async fn get_player_stats(
    State(app): State<AppState>,
    Path(player_id): Path<String>,
) -> Result<Json<PlayerStatsInfo>, ServiceError> {
    let player_id = PlayerId(
        Uuid::parse_str(&player_id)
            .map_err(|_| ServiceError::BadRequest("Invalid player ID".to_string()))?,
    );
    let stats = app
        .app
        .get_stats_use_case
        .get_stats(player_id)
        .await
        .map_err(|_| ServiceError::Internal("Failed to retrieve player stats".to_string()))?;

    Ok(Json(stats.into()))
}

pub async fn get_player_by_username(
    State(app): State<AppState>,
    Path(username): Path<String>,
) -> Result<Json<PlayerInfo>, ServiceError> {
    let Some(account) = app.auth.get_account_by_username(&username).await else {
        return Err(ServiceError::NotFound("Player not found".to_string()));
    };

    let player_id = app
        .app
        .player_resolver_service
        .resolve_player_id_by_account_id(&account.account_id)
        .await
        .map_err(|_| ServiceError::Internal("Failed to resolve player ID".to_string()))?;

    get_player_info_helper(&app, player_id).await.map(Json)
}

pub async fn get_player_by_account_id(
    State(app): State<AppState>,
    Path(account_id): Path<String>,
) -> Result<Json<PlayerInfo>, ServiceError> {
    let account_id = AccountId(account_id);
    let Some(account) = app.auth.get_account(&account_id).await else {
        return Err(ServiceError::NotFound("Player not found".to_string()));
    };

    let player_id = app
        .app
        .player_resolver_service
        .resolve_player_id_by_account_id(&account.account_id)
        .await
        .map_err(|_| ServiceError::Internal("Failed to resolve player ID".to_string()))?;

    get_player_info_helper(&app, player_id).await.map(Json)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub struct RatingResponse {
    rating: f64,
    participation_rating: f64,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerInfo {
    pub id: String,
    pub account_id: String,
    pub username: String,
    pub display_name: String,
    pub rating: Option<RatingResponse>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerStatsInfo {
    pub games_played: u32,
    pub rated_games_played: u32,
    pub games_won: u32,
    pub games_lost: u32,
    pub games_drawn: u32,
}

impl From<PlayerStatsView> for PlayerStatsInfo {
    fn from(stats: PlayerStatsView) -> Self {
        PlayerStatsInfo {
            games_played: stats.games_played,
            rated_games_played: stats.rated_games_played,
            games_won: stats.games_won,
            games_lost: stats.games_lost,
            games_drawn: stats.games_drawn,
        }
    }
}
