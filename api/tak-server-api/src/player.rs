use axum::{
    Json,
    extract::{Path, State},
};
use tak_server_app::domain::PlayerId;
use uuid::Uuid;

use crate::{AppState, ServiceError};

pub async fn get_player_info(
    State(app): State<AppState>,
    Path(player_id): Path<String>,
) -> Result<Json<PlayerInfo>, ServiceError> {
    let player_id = PlayerId(
        Uuid::parse_str(&player_id)
            .map_err(|_| ServiceError::BadRequest("Invalid player ID".to_string()))?,
    );
    let rating = app
        .app
        .player_get_rating_use_case
        .get_rating(player_id)
        .await;
    let rating = match rating {
        Ok(Some(rating_view)) => RatingResponse::Rated {
            rating: rating_view.rating,
            max_rating: rating_view.max_rating,
            rated_games_played: rating_view.rated_games_played,
            participation_rating: rating_view.participation_rating,
        },
        Ok(None) => RatingResponse::Unrated,
        Err(_) => {
            return Err(ServiceError::Internal(
                "Failed to retrieve player rating".to_string(),
            ));
        }
    };
    let account = app
        .app
        .get_account_workflow
        .get_account(player_id)
        .await
        .map_err(|_| ServiceError::Internal("Failed to retrieve player account".to_string()))?;

    Ok(Json(PlayerInfo {
        id: player_id.to_string(),
        username: account.username,
        display_name: account.display_name,
        rating,
    }))
}

#[derive(serde::Serialize)]
#[serde(
    rename_all = "camelCase",
    tag = "type",
    rename_all_fields = "camelCase"
)]
pub enum RatingResponse {
    Unrated,
    Rated {
        rating: f64,
        max_rating: f64,
        rated_games_played: u32,
        participation_rating: f64,
    },
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerInfo {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub rating: RatingResponse,
}
