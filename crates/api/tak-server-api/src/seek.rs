use axum::{
    Json,
    extract::{Path, State},
    routing::{delete, get, post},
};
use tak_core::TakPlayer;
use tak_server_api_contract::{
    game::JsonGameSettings,
    seek::{CreateSeekPayload, JsonSeek},
};
use tak_server_app::{
    domain::{SeekId, seek::CreateSeekError},
    services::player_resolver::ResolveError,
    workflow::matchmaking::{SeekView, accept::AcceptSeekError},
};

use crate::{AppState, ServiceError, auth::Auth};

pub fn register_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", get(get_seeks))
        .route("/", post(create_seek))
        .route("/{seek_id}", delete(cancel_seek))
        .route("/{seek_id}/accept", post(accept_seek))
}

pub async fn get_seeks(State(app): State<AppState>) -> Json<Vec<JsonSeek>> {
    let seeks = app.app.seek_list_use_case.list_seeks();
    Json(seeks.into_iter().map(|seek| from_seek_view(seek)).collect())
}

pub async fn create_seek(
    auth: Auth,
    State(app): State<AppState>,
    Json(payload): Json<CreateSeekPayload>,
) -> Result<Json<JsonSeek>, ServiceError> {
    let player_id = match app
        .app
        .player_resolver_service
        .resolve_player_id_by_account_id(&auth.account.account_id)
        .await
    {
        Ok(id) => id,
        Err(ResolveError::Internal) => {
            return Err(ServiceError::Internal(
                "Failed to resolve player ID".to_string(),
            ));
        }
    };

    let color = match payload.color.as_str() {
        "white" => Some(TakPlayer::White),
        "black" => Some(TakPlayer::Black),
        "random" => None,
        _ => {
            return Err(ServiceError::BadRequest("Invalid color choice".to_string()));
        }
    };

    let game_settings = payload.game_settings.to_game_settings();

    match app.app.seek_create_use_case.create_seek(
        player_id,
        color,
        game_settings,
        payload.is_rated,
    ) {
        Ok(seek) => Ok(Json(from_seek_view(seek))),
        Err(CreateSeekError::InvalidGameSettings) => Err(ServiceError::BadRequest(
            "Invalid game settings".to_string(),
        )),
    }
}

pub async fn cancel_seek(
    auth: Auth,
    State(app): State<AppState>,
    Path(seek_id): Path<u64>,
) -> Result<(), ServiceError> {
    let player_id = match app
        .app
        .player_resolver_service
        .resolve_player_id_by_account_id(&auth.account.account_id)
        .await
    {
        Ok(id) => id,
        Err(ResolveError::Internal) => {
            return Err(ServiceError::Internal(
                "Failed to resolve player ID".to_string(),
            ));
        }
    };
    if !app
        .app
        .seek_cancel_use_case
        .cancel_seek(player_id, SeekId(seek_id))
    {
        return Err(ServiceError::NotFound("Seek not found".to_string()));
    }
    Ok(())
}

pub async fn accept_seek(
    auth: Auth,
    State(app): State<AppState>,
    Path(seek_id): Path<u64>,
) -> Result<(), ServiceError> {
    let player_id = match app
        .app
        .player_resolver_service
        .resolve_player_id_by_account_id(&auth.account.account_id)
        .await
    {
        Ok(id) => id,
        Err(ResolveError::Internal) => {
            return Err(ServiceError::Internal(
                "Failed to resolve player ID".to_string(),
            ));
        }
    };
    match app
        .app
        .seek_accept_use_case
        .accept_seek(player_id, SeekId(seek_id))
        .await
    {
        Ok(_) => Ok(()),
        Err(AcceptSeekError::SeekNotFound) => {
            Err(ServiceError::NotFound("Seek not found".to_string()))
        }
        Err(AcceptSeekError::FailedToCreateGame) => {
            Err(ServiceError::Internal("Failed to accept seek".to_string()))
        }
    }
}

pub fn from_seek_view(seek: SeekView) -> JsonSeek {
    JsonSeek {
        id: seek.id.to_string(),
        creator_id: seek.creator_id.to_string(),
        color: match seek.color {
            None => "random".to_string(),
            Some(TakPlayer::White) => "white".to_string(),
            Some(TakPlayer::Black) => "black".to_string(),
        },
        game_settings: JsonGameSettings::from_game_settings(&seek.game_settings),
        is_rated: seek.is_rated,
    }
}
