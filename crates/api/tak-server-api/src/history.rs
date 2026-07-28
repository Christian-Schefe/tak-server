use axum::{
    Json,
    extract::{Query, State},
    routing::get,
};
use tak_server_api_contract::game::JsonEndedGameInfo;
use tak_server_app::{
    domain::{
        Pagination, SortOrder,
        game_history::{GameQuery, GameSortBy},
    },
    workflow::history::query::GameQueryError,
};

use crate::{AppState, PaginatedResponse, PaginationQuery, ServiceError, game::from_game_record};

pub fn register_routes() -> axum::Router<AppState> {
    axum::Router::new().route("/", get(query_game_history))
}

pub async fn query_game_history(
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
