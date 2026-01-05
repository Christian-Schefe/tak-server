use std::{sync::LazyLock, time::Instant};

use axum::{
    Json,
    extract::{Path, State},
};
use tak_server_app::{
    domain::{
        Pagination, RepoError, SortOrder,
        rating::{RatingQuery, RatingSortBy},
    },
    workflow::account::get_account::GetAccountError,
};
use tokio::sync::Mutex;

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
    let (player_id, account) = match app_state
        .acl
        .get_account_and_player_id_by_username(&name)
        .await
    {
        Some(res) => res,
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

async fn query_ratings(
    app_state: AppState,
    query: RatingQuery,
) -> Result<(Vec<JsonPlayerRatingResponse>, usize), ServiceError> {
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
        .filter_map(|(rating, account)| match account {
            Ok(account) => Some((rating, account.username.clone(), account.is_bot())),
            Err(GetAccountError::AccountNotFound) => None,
            Err(GetAccountError::RepositoryError) => {
                log::error!(
                    "Failed to get account for player {}: Repository error",
                    rating.player_id,
                );
                None
            }
        })
        .collect::<Vec<_>>();

    let ratings: Vec<JsonPlayerRatingResponse> = ratings_with_usernames
        .into_iter()
        .map(|(rating, username, is_bot)| JsonPlayerRatingResponse {
            isbot: is_bot,
            name: username,
            rating: rating.rating.round(),
            ratedgames: rating.rated_games_played as i32,
            maxrating: rating.max_rating.round(),
            participation_rating: rating.participation_rating.round(),
        })
        .collect();

    let total = res.total_count;
    Ok((ratings, total))
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
                "max_rating" => Some(Ok(RatingSortBy::MaxRating)),
                "rated_games" => Some(Ok(RatingSortBy::RatedGames)),
                _ => Some(Err(ServiceError::BadRequest(
                    "Invalid sort order".to_string(),
                ))),
            }
        })
        .transpose()?
        .unwrap_or(RatingSortBy::Rating);

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

    let (ratings, total) = query_ratings(app_state, query).await?;

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

static RATING_LIST_CACHE: LazyLock<Mutex<Option<(Json<Vec<serde_json::Value>>, Instant)>>> =
    LazyLock::new(|| Mutex::new(None));

pub async fn get_rating_list(
    State(app_state): State<AppState>,
) -> Result<Json<Vec<serde_json::Value>>, ServiceError> {
    let mut lock = RATING_LIST_CACHE.lock().await;
    let ratings = if let Some((cached_ratings, timestamp)) = &*lock
        && timestamp.elapsed().as_secs() < 60 * 5
    {
        cached_ratings.clone()
    } else {
        let query = RatingQuery {
            pagination: Pagination {
                offset: None,
                limit: None,
            },
            sort: Some((SortOrder::Descending, RatingSortBy::Rating)),
        };
        let (ratings, _) = query_ratings(app_state, query).await?;
        let res: Json<Vec<serde_json::Value>> = Json(
            ratings
                .into_iter()
                .map(|item| {
                    serde_json::json!([
                        item.name,
                        item.rating.round(),
                        item.participation_rating.round(),
                        item.ratedgames,
                        if item.isbot { 1 } else { 0 }
                    ])
                })
                .collect(),
        );
        *lock = Some((res.clone(), Instant::now()));
        res
    };
    Ok(ratings)
}

#[derive(serde::Deserialize)]
pub struct JsonRatingsFilter {
    limit: Option<usize>,
    page: Option<usize>,
    skip: Option<usize>,
    order: Option<String>,
    sort: Option<String>,
}
