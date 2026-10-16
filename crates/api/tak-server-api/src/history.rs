use axum::{
    Json,
    extract::{Path, Query, State},
    routing::get,
};
use tak_domain::{
    Pagination, SortOrder, UserId,
    game_history::{GamePlayerFilter, GameQuery, GameSortBy},
};
use tak_server_api_contract::game::JsonEndedGameInfo;
use tak_server_app::workflow::history::query::GameQueryError;

use crate::{AppState, PaginatedResponse, PaginationQuery, ServiceError, game::from_game_record};

pub fn register_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", get(query_game_history))
        .route("/player/{user_id}", get(query_player_game_history))
}

async fn query_game_history(
    State(app): State<AppState>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<PaginatedResponse<JsonEndedGameInfo>>, ServiceError> {
    let filter = GameQuery {
        pagination: Pagination::new(pagination.page.saturating_sub(1), pagination.page_size),
        sort: Some((SortOrder::Descending, GameSortBy::Date)),
        ..Default::default()
    };
    let history = app
        .app
        .game_history_query_use_case
        .query_games(filter)
        .await
        .map_err(|GameQueryError::RepositoryError| {
            ServiceError::Internal("Failed to retrieve game history".to_string())
        })?;

    Ok(Json(PaginatedResponse {
        items: history.items.iter().map(from_game_record).collect(),
        total_count: history.total_count as u32,
    }))
}

async fn query_player_game_history(
    State(app): State<AppState>,
    Path(user_id): Path<String>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<PaginatedResponse<JsonEndedGameInfo>>, ServiceError> {
    let user_id = UserId::try_from(user_id)
        .map_err(|_| ServiceError::BadRequest("Invalid user ID".to_string()))?;
    let filter = GameQuery {
        pagination: Pagination::new(pagination.page.saturating_sub(1), pagination.page_size),
        sort: Some((SortOrder::Descending, GameSortBy::Date)),
        player_filters: vec![(GamePlayerFilter::UserId(user_id), None)],
        ..Default::default()
    };
    let history = app
        .app
        .game_history_query_use_case
        .query_games(filter)
        .await
        .map_err(|GameQueryError::RepositoryError| {
            ServiceError::Internal("Failed to retrieve game history".to_string())
        })?;

    Ok(Json(PaginatedResponse {
        items: history.items.iter().map(from_game_record).collect(),
        total_count: history.total_count as u32,
    }))
}
