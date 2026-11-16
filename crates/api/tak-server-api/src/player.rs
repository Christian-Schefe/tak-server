use axum::{
    Json,
    extract::{Path, Query, State},
    routing::get,
};
use chrono::{DateTime, Utc};
use tak_domain::{
    UserId,
    moderation::{UserRole, UserType},
};
use tak_server_api_contract::player::{UserInfoResponse, UserRoleResponse, UserTypeResponse};
use tak_server_app::{
    ports::{Pagination, rating::RatingQuery},
    workflow::{account::get_snapshot::GetSnapshotError, player::PlayerStatsView},
};
use uuid::Uuid;

use crate::{AppState, PaginatedResponse, PaginationQuery, ServiceError};

pub fn register_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/player/{user_id}", get(get_user_info))
        .route("/player/{user_id}/stats", get(get_player_stats))
        .route("/player/{user_id}/rating-history", get(get_rating_history))
        .route("/leaderboard", get(query_player_ratings))
        .route("/online", get(get_online_users))
}

async fn get_online_users(State(app): State<AppState>) -> Json<Vec<String>> {
    let users = app.app.account_get_online_use_case.get_online_users();
    Json(users.into_iter().map(|a| a.to_string()).collect())
}

async fn get_user_info(
    State(app): State<AppState>,
    Path(user_id): Path<String>,
) -> Result<Json<UserInfoResponse>, ServiceError> {
    let user_id = UserId(
        Uuid::parse_str(&user_id)
            .map_err(|_| ServiceError::BadRequest("Invalid user ID".to_string()))?,
    );
    let snapshot = match app
        .app
        .get_snapshot_workflow
        .get_snapshot(user_id, Utc::now())
        .await
    {
        Ok(snapshot) => snapshot,
        Err(GetSnapshotError::NotFound) => {
            return Err(ServiceError::NotFound(
                "Player account not found".to_string(),
            ));
        }
        Err(GetSnapshotError::Internal) => {
            return Err(ServiceError::Internal(
                "Failed to retrieve player account".to_string(),
            ));
        }
    };
    let moderation = match app
        .app
        .account_moderate_use_case
        .get_moderation(user_id)
        .await
    {
        Ok(moderation) => moderation,
        Err(_) => {
            return Err(ServiceError::Internal(
                "Failed to retrieve moderation info".to_string(),
            ));
        }
    };

    Ok(Json(UserInfoResponse {
        user_id: user_id.to_string(),
        username: snapshot.username,
        participation_rating: snapshot.rating,
        user_role: match moderation.role {
            UserRole::User => UserRoleResponse::User,
            UserRole::Moderator => UserRoleResponse::Moderator,
            UserRole::Admin => UserRoleResponse::Admin,
        },
        user_type: match moderation.user_type {
            UserType::Guest => UserTypeResponse::Guest,
            UserType::Bot => UserTypeResponse::Bot,
            UserType::Player => UserTypeResponse::Player,
        },
    }))
}

async fn get_player_stats(
    State(app): State<AppState>,
    Path(user_id): Path<String>,
) -> Result<Json<PlayerStatsInfoResponse>, ServiceError> {
    let user_id = UserId(
        Uuid::parse_str(&user_id)
            .map_err(|_| ServiceError::BadRequest("Invalid user ID".to_string()))?,
    );
    let stats = app
        .app
        .get_stats_use_case
        .get_stats(user_id)
        .await
        .map_err(|_| ServiceError::Internal("Failed to retrieve player stats".to_string()))?;

    Ok(Json(stats.into()))
}

async fn get_rating_history(
    State(app): State<AppState>,
    Path(user_id): Path<String>,
    Query(query): Query<RatingHistoryQuery>,
) -> Result<Json<RatingHistoryResponse>, ServiceError> {
    let user_id = UserId(
        Uuid::parse_str(&user_id)
            .map_err(|_| ServiceError::BadRequest("Invalid user ID".to_string()))?,
    );
    match app
        .app
        .player_get_rating_use_case
        .get_rating_history(user_id, query.from.map(|t| t.0), query.to.map(|t| t.0))
        .await
    {
        Ok(history) => Ok(Json(RatingHistoryResponse {
            entries: history
                .entries
                .into_iter()
                .map(|entry| RatingHistoryResponseEntry {
                    timestamp: entry.timestamp,
                    rating: entry.rating,
                })
                .collect(),
            first_entry_before_range: history.first_entry_before_range.map(|entry| {
                RatingHistoryResponseEntry {
                    timestamp: entry.timestamp,
                    rating: entry.rating,
                }
            }),
        })),
        Err(_) => Err(ServiceError::Internal(
            "Failed to retrieve rating history".to_string(),
        )),
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct RatingResponseEntry {
    user_id: String,
    rating: f64,
}

async fn query_player_ratings(
    State(app): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<PaginatedResponse<RatingResponseEntry>>, ServiceError> {
    let query = RatingQuery {
        pagination: Pagination::new(query.page.saturating_sub(1), query.page_size),
        ..Default::default()
    };
    match app
        .app
        .player_get_rating_use_case
        .query_player_ratings(query)
        .await
    {
        Ok(response) => Ok(Json(PaginatedResponse {
            items: response
                .items
                .into_iter()
                .map(|(user_id, rating)| RatingResponseEntry {
                    user_id: user_id.to_string(),
                    rating,
                })
                .collect(),
            total_count: response.total_count as u32,
        })),
        Err(_) => Err(ServiceError::Internal(
            "Failed to retrieve player ratings".to_string(),
        )),
    }
}

#[derive(serde::Deserialize)]
struct RatingHistoryQuery {
    from: Option<SerdeTimestamp>,
    to: Option<SerdeTimestamp>,
}

#[derive(serde::Deserialize)]
struct SerdeTimestamp(#[serde(with = "chrono::serde::ts_milliseconds")] DateTime<Utc>);

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct RatingHistoryResponse {
    entries: Vec<RatingHistoryResponseEntry>,
    first_entry_before_range: Option<RatingHistoryResponseEntry>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct RatingHistoryResponseEntry {
    #[serde(with = "chrono::serde::ts_milliseconds")]
    timestamp: DateTime<Utc>,
    rating: f64,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PlayerStatsInfoResponse {
    ranking: Option<PlayerRankingInfoResponse>,
    games_played: u32,
    rated_games_played: u32,
    games_won: u32,
    games_lost: u32,
    games_drawn: u32,
    win_streak: u32,
    longest_win_streak: u32,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PlayerRankingInfoResponse {
    rating: f64,
    max_rating: f64,
    rank: u32,
}

impl From<PlayerStatsView> for PlayerStatsInfoResponse {
    fn from(stats: PlayerStatsView) -> Self {
        PlayerStatsInfoResponse {
            games_played: stats.games_played,
            rated_games_played: stats.rated_games_played,
            games_won: stats.games_won,
            games_lost: stats.games_lost,
            games_drawn: stats.games_drawn,
            win_streak: stats.win_streak,
            longest_win_streak: stats.longest_win_streak,
            ranking: stats.ranking.map(|ranking| PlayerRankingInfoResponse {
                rating: ranking.rating,
                max_rating: ranking.max_rating,
                rank: ranking.ranking,
            }),
        }
    }
}
