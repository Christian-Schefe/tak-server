use axum::{
    Json,
    extract::{Path, Query, State},
    routing::get,
};
use chrono::{DateTime, Utc};
use tak_server_app::{
    domain::{AccountId, Pagination, PlayerId, rating::RatingQuery},
    workflow::{
        account::get_account::GetAccountError,
        player::{PlayerStatsView, get_rating::GetRatingError},
    },
};
use uuid::Uuid;

use crate::{AppState, PaginatedResponse, PaginationQuery, ServiceError};

pub fn register_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/account/{account_id}", get(get_player_by_account_id))
        .route("/player/{player_id}", get(get_player_info))
        .route("/player/{player_id}/stats", get(get_player_stats))
        .route(
            "/player/{player_id}/rating-history",
            get(get_rating_history),
        )
        .route("/leaderboard", get(query_player_ratings))
}

async fn get_player_info_helper(
    app: &AppState,
    player_id: PlayerId,
) -> Result<PlayerInfo, ServiceError> {
    let (account_id, account) = match app.app.get_account_workflow.get_account(player_id).await {
        Ok(account) => account,
        Err(GetAccountError::AccountNotFound) => {
            return Err(ServiceError::NotFound("Player not found".to_string()));
        }
        Err(GetAccountError::RepositoryError) => {
            return Err(ServiceError::Internal(
                "Failed to retrieve player account".to_string(),
            ));
        }
    };

    let participation_rating = match app
        .app
        .player_get_rating_use_case
        .get_rating(player_id)
        .await
    {
        Ok(rating) => rating,
        Err(GetRatingError::Internal) => {
            return Err(ServiceError::Internal(
                "Failed to retrieve player rating".to_string(),
            ));
        }
    };

    Ok(PlayerInfo {
        player_id: player_id.to_string(),
        account_id: account_id.to_string(),
        username: account.username,
        participation_rating,
    })
}

async fn get_player_info(
    State(app): State<AppState>,
    Path(player_id): Path<String>,
) -> Result<Json<PlayerInfo>, ServiceError> {
    let player_id = PlayerId(
        Uuid::parse_str(&player_id)
            .map_err(|_| ServiceError::BadRequest("Invalid player ID".to_string()))?,
    );
    get_player_info_helper(&app, player_id).await.map(Json)
}

async fn get_player_stats(
    State(app): State<AppState>,
    Path(player_id): Path<String>,
) -> Result<Json<JsonPlayerStatsInfo>, ServiceError> {
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

async fn get_player_by_account_id(
    State(app): State<AppState>,
    Path(account_id): Path<String>,
) -> Result<Json<PlayerInfo>, ServiceError> {
    let Ok(account_id) = AccountId::try_from(account_id) else {
        return Err(ServiceError::BadRequest(
            "Invalid account ID format".to_string(),
        ));
    };
    let Some(_) = app.auth.get_account(&account_id).await else {
        return Err(ServiceError::NotFound("Account not found".to_string()));
    };

    let player_id = app
        .app
        .player_resolver_service
        .resolve_player_id_by_account_id(&account_id)
        .await
        .map_err(|_| ServiceError::Internal("Failed to resolve player ID".to_string()))?;

    get_player_info_helper(&app, player_id).await.map(Json)
}

async fn get_rating_history(
    State(app): State<AppState>,
    Path(player_id): Path<String>,
    Query(query): Query<RatingHistoryQuery>,
) -> Result<Json<JsonRatingHistory>, ServiceError> {
    let player_id = PlayerId(
        Uuid::parse_str(&player_id)
            .map_err(|_| ServiceError::BadRequest("Invalid player ID".to_string()))?,
    );
    match app
        .app
        .player_get_rating_use_case
        .get_rating_history(player_id, query.from.map(|t| t.0), query.to.map(|t| t.0))
        .await
    {
        Ok(history) => Ok(Json(JsonRatingHistory {
            entries: history
                .entries
                .into_iter()
                .map(|entry| JsonRatingHistoryEntry {
                    timestamp: entry.timestamp,
                    rating: entry.rating,
                })
                .collect(),
            first_entry_before_range: history.first_entry_before_range.map(|entry| {
                JsonRatingHistoryEntry {
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
struct JsonRatingEntry {
    player_id: String,
    rating: f64,
}

async fn query_player_ratings(
    State(app): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<PaginatedResponse<JsonRatingEntry>>, ServiceError> {
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
                .map(|(player_id, rating)| JsonRatingEntry {
                    player_id: player_id.to_string(),
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
struct JsonRatingHistory {
    entries: Vec<JsonRatingHistoryEntry>,
    first_entry_before_range: Option<JsonRatingHistoryEntry>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonRatingHistoryEntry {
    #[serde(with = "chrono::serde::ts_milliseconds")]
    timestamp: DateTime<Utc>,
    rating: f64,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PlayerInfo {
    player_id: String,
    account_id: String,
    username: String,
    participation_rating: Option<f64>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonPlayerStatsInfo {
    ranking: Option<JsonPlayerRankingInfo>,
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
struct JsonPlayerRankingInfo {
    rating: f64,
    max_rating: f64,
    rank: u32,
}

impl From<PlayerStatsView> for JsonPlayerStatsInfo {
    fn from(stats: PlayerStatsView) -> Self {
        JsonPlayerStatsInfo {
            games_played: stats.games_played,
            rated_games_played: stats.rated_games_played,
            games_won: stats.games_won,
            games_lost: stats.games_lost,
            games_drawn: stats.games_drawn,
            win_streak: stats.win_streak,
            longest_win_streak: stats.longest_win_streak,
            ranking: stats.ranking.map(|ranking| JsonPlayerRankingInfo {
                rating: ranking.rating,
                max_rating: ranking.max_rating,
                rank: ranking.ranking,
            }),
        }
    }
}
