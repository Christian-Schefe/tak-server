use axum::{
    Json,
    extract::{Path, State},
};
use tak_server_app::domain::{Pagination, SortOrder};

use crate::{
    app::ServiceError,
    http::{AppState, PaginatedResponse},
};

#[derive(serde::Serialize, Clone)]
pub struct JsonPlayerRatingResponse {
    name: String,
    rating: f64,
    ratedgames: i32,
    maxrating: f64,
    participation_rating: f64,
    isbot: bool,
}

pub async fn get_rating_by_name(
    Path(name): Path<String>,
    State(app_state): State<AppState>,
) -> Result<Json<JsonPlayerRatingResponse>, ServiceError> {
    let player_id = match app_state.acl.get_player_id_by_username(&name).await {
        Some(id) => id,
        None => {
            return Err(ServiceError::NotFound(format!(
                "Player with name '{}' not found",
                name
            )));
        }
    };
    let rating = match app_state
        .app
        .player_get_rating_use_case
        .get_rating(player_id)
        .await
    {
        Some(rating) => rating,
        None => {
            return Err(ServiceError::NotFound(format!(
                "Rating for player '{}' not found",
                name
            )));
        }
    };
    let account_id = match app_state
        .app
        .player_resolver_service
        .resolve_account_id_by_player_id(player_id)
        .await
    {
        Ok(account) => account,
        Err(()) => {
            return Err(ServiceError::NotFound(format!(
                "Account for player '{}' not found",
                name
            )));
        }
    };
    let Some(account) = app_state.auth.get_account(account_id).await else {
        return Err(ServiceError::NotFound(format!(
            "Account for player '{}' not found",
            name
        )));
    };
    let rating = JsonPlayerRatingResponse {
        name: name.clone(),
        rating: rating.rating,
        ratedgames: rating.rated_games_played as i32,
        maxrating: rating.max_rating,
        participation_rating: rating.participation_rating,
        isbot: account.is_bot(),
    };

    Ok(Json(rating))
}

pub async fn get_ratings(
    State(app_state): State<AppState>,
    Json(filter): Json<JsonRatingsFilter>,
) -> Result<Json<PaginatedResponse<JsonPlayerRatingResponse>>, ServiceError> {
    let page = filter.page.unwrap_or(0);
    let limit = filter.limit.filter(|&l| l > 0).unwrap_or(50);
    let skip = filter.skip.unwrap_or(0);
    let offset = if page > 1 { (page - 1) * limit } else { skip };
    let pagination = Pagination {
        offset: Some(offset),
        limit: Some(limit),
    };
    let sort = filter
        .sort
        .as_ref()
        .and_then(|sort_str| {
            let sort_str = sort_str.trim().to_lowercase();
            match sort_str.as_str() {
                "" => None,
                "rating" => Some(Ok(PlayerSortBy::Rating)),
                "participation_rating" => Some(Ok(PlayerSortBy::ParticipationRating)),
                "id" => Some(Ok(PlayerSortBy::PlayerId)),
                _ => Some(Err(ServiceError::BadRequest(
                    "Invalid sort order".to_string(),
                ))),
            }
        })
        .transpose()?
        .unwrap_or(PlayerSortBy::ParticipationRating);

    let order = filter
        .order
        .as_ref()
        .and_then(|order_str| match order_str.trim().to_lowercase().as_str() {
            "asc" => Some(Ok(SortOrder::Ascending)),
            "desc" => Some(Ok(SortOrder::Descending)),
            "" => None,
            _ => Some(Err(ServiceError::BadRequest(
                "Invalid sort order".to_string(),
            ))),
        })
        .transpose()?
        .unwrap_or(SortOrder::Descending);

    let filter = PlayerFilter {
        pagination,
        sort: Some((order, sort)),
        id: filter.id.map(|id| id.into()),
        username: filter.name,
        ..Default::default()
    };

    let res: PlayerFilterResult = app_state.player_service.get_players(filter).await?;

    let ratings: Vec<JsonPlayerRatingResponse> = res
        .players
        .into_iter()
        .map(|(_, player)| JsonPlayerRatingResponse {
            name: player.username.clone(),
            rating: player.rating.rating,
            ratedgames: player.rating.rated_games_played as i32,
            maxrating: player.rating.max_rating,
            participation_rating: player.rating.participation_rating,
            isbot: player.flags.is_bot,
        })
        .collect();

    let total = res.total_count;
    Ok(Json(PaginatedResponse {
        items: ratings,
        total,
        page,
        per_page: limit,
        total_pages: if limit > 0 {
            (total + limit - 1) / limit
        } else {
            1
        },
    }))
}

#[derive(serde::Deserialize)]
pub struct JsonRatingsFilter {
    limit: Option<usize>,
    page: Option<usize>,
    skip: Option<usize>,
    order: Option<String>,
    sort: Option<String>,
    id: Option<i64>,
    name: Option<String>,
}
