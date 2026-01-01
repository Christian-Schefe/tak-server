use axum::{
    Json,
    extract::{Path, State},
};
use tak_server_app::domain::{
    Pagination, RepoError, SortOrder,
    rating::{RatingQuery, RatingSortBy},
};

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
    let Some(account) = app_state.auth.get_account(&account_id).await else {
        return Err(ServiceError::NotFound(format!(
            "Account for player '{}' not found",
            name
        )));
    };
    let rating = JsonPlayerRatingResponse {
        name: name.clone(),
        rating: rating.rating.round(),
        ratedgames: rating.rated_games_played as i32,
        maxrating: rating.max_rating.round(),
        participation_rating: rating.participation_rating.round(),
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
                "rating" => Some(Ok(RatingSortBy::Rating)),
                "participation_rating" => Some(Ok(RatingSortBy::ParticipationRating)),
                "max_rating" => Some(Ok(RatingSortBy::MaxRating)),
                "rated_games" => Some(Ok(RatingSortBy::RatedGames)),
                _ => Some(Err(ServiceError::BadRequest(
                    "Invalid sort order".to_string(),
                ))),
            }
        })
        .transpose()?
        .unwrap_or(RatingSortBy::ParticipationRating);

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

    let query = RatingQuery {
        pagination,
        sort: Some((order, sort)),
        ..Default::default()
    };

    let res = match app_state
        .app
        .player_get_rating_use_case
        .query_ratings(query)
        .await
    {
        Ok(result) => result,
        Err(RepoError::StorageError(_)) => {
            return Err(ServiceError::Internal("Error querying ratings".to_string()));
        }
    };

    let mut username_futures = Vec::new();
    for rating in res.items.iter() {
        let username = app_state
            .app
            .get_account_workflow
            .get_account(rating.player_id);
        username_futures.push(username);
    }
    let usernames = futures::future::join_all(username_futures).await;
    let ratings_with_usernames = res
        .items
        .into_iter()
        .zip(usernames.into_iter())
        .filter_map(|(rating, account)| account.ok().map(|account| (rating, account)))
        .collect::<Vec<_>>();

    let ratings: Vec<JsonPlayerRatingResponse> = ratings_with_usernames
        .into_iter()
        .map(|(rating, account)| JsonPlayerRatingResponse {
            isbot: account.is_bot(),
            name: account.username,
            rating: rating.rating.round(),
            ratedgames: rating.rated_games_played as i32,
            maxrating: rating.max_rating.round(),
            participation_rating: rating.participation_rating.round(),
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
}
