use std::str::FromStr;

use axum::{
    Json,
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::header,
    response::Response,
};
use chrono::{DateTime, Utc};
use tak_server_api_contract::game::JsonEndedGameInfo;
use tak_server_app::{
    domain::{
        AccountId, Pagination, PlayerId, SortOrder,
        game_history::{GamePlayerFilter, GameQuery, GameSortBy},
        profile::ProfilePictureFileType,
    },
    workflow::{account::AccountProfileView, player::PlayerStatsView},
};
use uuid::Uuid;

use crate::{
    AppState, PaginatedResponse, PaginationQuery, ServiceError, auth::Auth, game::from_game_record,
};

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
    let Some(account_id) = AccountId::from_string(account_id) else {
        return Err(ServiceError::BadRequest(
            "Invalid account ID format".to_string(),
        ));
    };
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

pub async fn get_account_profile(
    State(app): State<AppState>,
    Path(account_id): Path<String>,
) -> Result<Json<PlayerProfileInfo>, ServiceError> {
    let Some(account_id) = AccountId::from_string(account_id) else {
        return Err(ServiceError::BadRequest(
            "Invalid account ID format".to_string(),
        ));
    };
    let Some(account) = app.auth.get_account(&account_id).await else {
        return Err(ServiceError::NotFound("Player not found".to_string()));
    };
    let profile = app
        .app
        .get_profile_use_case
        .get_profile(&account.account_id)
        .await
        .map_err(|_| ServiceError::Internal("Failed to retrieve player profile".to_string()))?;

    Ok(Json(profile.into()))
}

pub async fn update_account_profile(
    auth: Auth,
    State(app): State<AppState>,
    Json(payload): Json<PlayerProfileUpdate>,
) -> Result<(), ServiceError> {
    let country = match payload.country {
        Some(country_str) => Some(
            country_code_enum::CountryCode::from_str(&country_str)
                .map_err(|_| ServiceError::BadRequest("Invalid country code".to_string()))?,
        ),
        None => None,
    };

    app.app
        .update_profile_use_case
        .update_profile(&auth.account.account_id, country)
        .await
        .map_err(|_| ServiceError::Internal("Failed to update player profile".to_string()))
}

#[derive(serde::Deserialize)]
#[allow(dead_code)]
#[serde(rename_all = "camelCase")]
pub struct VersionQuery {
    #[serde(rename = "v")]
    version: Option<u64>,
}

pub async fn get_profile_picture(
    State(app): State<AppState>,
    Path(account_id): Path<String>,
    Query(_version_query): Query<VersionQuery>,
) -> Result<Response, ServiceError> {
    let Some(account_id) = AccountId::from_string(account_id) else {
        return Err(ServiceError::BadRequest(
            "Invalid account ID format".to_string(),
        ));
    };
    let Some(account) = app.auth.get_account(&account_id).await else {
        return Err(ServiceError::NotFound("Player not found".to_string()));
    };
    let profile_picture = app
        .app
        .get_profile_use_case
        .get_profile_picture(&account.account_id)
        .await
        .map_err(|_| ServiceError::Internal("Failed to retrieve profile picture".to_string()))?;
    let Some(profile_picture) = profile_picture else {
        return Err(ServiceError::NotFound(
            "Profile picture not found".to_string(),
        ));
    };
    let body = Body::from_stream(profile_picture.stream);
    let mut response = Response::new(body);
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static(match profile_picture.content_type {
            ProfilePictureFileType::WebP => "image/webp",
        }),
    );
    Ok(response)
}

pub async fn set_profile_picture(
    auth: Auth,
    State(app): State<AppState>,
    mut multipart: Multipart,
) -> Result<(), ServiceError> {
    while let Some(field) = match multipart.next_field().await {
        Ok(field) => field,
        Err(e) => {
            return Err(ServiceError::BadRequest(format!(
                "Failed to parse multipart: {}",
                e
            )));
        }
    } {
        let data = field.bytes().await.map_err(|e| {
            ServiceError::BadRequest(format!("Failed to read multipart field: {}", e))
        })?;

        let img = match image::load_from_memory(&data) {
            Ok(img) => img,
            Err(e) => {
                return Err(ServiceError::BadRequest(format!(
                    "Failed to parse image data: {}",
                    e
                )));
            }
        };

        match app
            .app
            .update_profile_use_case
            .set_profile_picture(&auth.account.account_id, img)
            .await
        {
            Ok(_) => (),
            Err(_) => {
                return Err(ServiceError::Internal(
                    "Failed to update profile picture".to_string(),
                ));
            }
        }
    }
    Ok(())
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
