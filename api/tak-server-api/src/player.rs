use axum::{
    Json,
    extract::{Path, Query, State},
    routing::get,
};
use chrono::{DateTime, Utc};
use tak_server_api_contract::game::JsonEndedGameInfo;
use tak_server_app::{
    domain::{
        AccountId, Pagination, PlayerId, SortOrder,
        game_history::{GamePlayerFilter, GameQuery, GameSortBy},
    },
    workflow::{
        account::{AccountProfileView, get_account::GetAccountError},
        player::{PlayerStatsView, get_rating::GetRatingError},
    },
};
use uuid::Uuid;

use crate::{AppState, PaginatedResponse, PaginationQuery, ServiceError, game::from_game_record};

pub fn register_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/username/{username}", get(get_player_by_username))
        .route("/account/{account_id}", get(get_player_by_account_id))
        .route("/player/{player_id}", get(get_player_info))
        .route("/player/{player_id}/stats", get(get_player_stats))
        .route(
            "/player/{player_id}/rating-history",
            get(get_rating_history),
        )
        .route("/player/{player_id}/games", get(get_games_history))
}

async fn get_player_info_helper(
    app: &AppState,
    player_id: PlayerId,
) -> Result<PlayerInfo, ServiceError> {
    let account = match app.app.get_account_workflow.get_account(player_id).await {
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
        account_id: account.account_id.to_string(),
        username: account.username,
        display_name: account.display_name,
        participation_rating,
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
    let Ok(account_id) = AccountId::try_from(account_id) else {
        return Err(ServiceError::BadRequest(
            "Invalid account ID format".to_string(),
        ));
    };
    let Some(account) = app.auth.get_account(&account_id).await else {
        return Err(ServiceError::NotFound("Account not found".to_string()));
    };

    let player_id = app
        .app
        .player_resolver_service
        .resolve_player_id_by_account_id(&account.account_id)
        .await
        .map_err(|_| ServiceError::Internal("Failed to resolve player ID".to_string()))?;

    get_player_info_helper(&app, player_id).await.map(Json)
}

pub async fn get_games_history(
    State(app): State<AppState>,
    Path(player_id): Path<String>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<PaginatedResponse<JsonEndedGameInfo>>, ServiceError> {
    let player_id = PlayerId(
        Uuid::parse_str(&player_id)
            .map_err(|_| ServiceError::BadRequest("Invalid player ID".to_string()))?,
    );
    let filter = GameQuery {
        player_filters: vec![(GamePlayerFilter::PlayerId(player_id), None)],
        pagination: Pagination::new(pagination.page, pagination.page_size),
        sort: Some((SortOrder::Descending, GameSortBy::Date)),
        ..Default::default()
    };
    match app
        .app
        .game_history_query_use_case
        .query_games(filter)
        .await
    {
        Ok(result) => Ok(Json(PaginatedResponse {
            items: result
                .items
                .into_iter()
                .map(|record| from_game_record(&record))
                .collect(),
            total_count: result.total_count as u32,
        })),
        Err(_) => Err(ServiceError::Internal(
            "Failed to retrieve game history".to_string(),
        )),
    }
}

pub async fn get_rating_history(
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

#[derive(serde::Deserialize)]
pub struct RatingHistoryQuery {
    pub from: Option<SerdeTimestamp>,
    pub to: Option<SerdeTimestamp>,
}

#[derive(serde::Deserialize)]
pub struct SerdeTimestamp(#[serde(with = "chrono::serde::ts_milliseconds")] pub DateTime<Utc>);

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonRatingHistory {
    pub entries: Vec<JsonRatingHistoryEntry>,
    pub first_entry_before_range: Option<JsonRatingHistoryEntry>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonRatingHistoryEntry {
    #[serde(with = "chrono::serde::ts_milliseconds")]
    timestamp: DateTime<Utc>,
    rating: f64,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerInfo {
    pub player_id: String,
    pub account_id: String,
    pub username: String,
    pub display_name: String,
    pub participation_rating: Option<f64>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonPlayerStatsInfo {
    pub ranking: Option<JsonPlayerRankingInfo>,
    pub games_played: u32,
    pub rated_games_played: u32,
    pub games_won: u32,
    pub games_lost: u32,
    pub games_drawn: u32,
    pub win_streak: u32,
    pub longest_win_streak: u32,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonPlayerRankingInfo {
    pub rating: f64,
    pub max_rating: f64,
    pub rank: u32,
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

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlayerProfileUpdate {
    pub country: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlayerProfileInfo {
    pub country: Option<String>,
    pub profile_picture_version: Option<u64>,
}

impl From<AccountProfileView> for PlayerProfileInfo {
    fn from(profile: AccountProfileView) -> Self {
        PlayerProfileInfo {
            country: profile.country.map(|c| c.to_string()),
            profile_picture_version: profile.profile_picture_version.map(|x| x.0),
        }
    }
}
