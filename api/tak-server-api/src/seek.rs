use axum::{Json, extract::State};
use tak_core::TakPlayer;
use tak_server_app::{
    domain::SeekId,
    services::player_resolver::ResolveError,
    workflow::matchmaking::{SeekView, accept::AcceptSeekError},
};

use crate::{AppState, ServiceError, auth::Auth, game::GameSettingsInfo};

pub async fn get_seeks(State(app): State<AppState>) -> Json<Vec<SeekInfo>> {
    let seeks = app.app.seek_list_use_case.list_seeks();
    Json(
        seeks
            .into_iter()
            .map(|seek| SeekInfo::from_seek_view(seek))
            .collect(),
    )
}

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AcceptSeekRequest {
    pub seek_id: u64,
}

pub async fn accept_seek(
    auth: Auth,
    State(app): State<AppState>,
    Json(request): Json<AcceptSeekRequest>,
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
        .accept_seek(player_id, SeekId(request.seek_id))
        .await
    {
        Ok(_) => Ok(()),
        Err(AcceptSeekError::SeekNotFound) => {
            Err(ServiceError::NotFound("Seek not found".to_string()))
        }
        Err(AcceptSeekError::InvalidOpponent) => Err(ServiceError::BadRequest(
            "You are not allowed to accept this seek".to_string(),
        )),
        Err(AcceptSeekError::FailedToCreateGame) => {
            Err(ServiceError::Internal("Failed to accept seek".to_string()))
        }
    }
}

#[derive(serde::Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SeekInfo {
    pub id: u64,
    pub creator_id: String,
    pub opponent_id: Option<String>,
    pub color: String,
    pub is_rated: bool,
    #[serde(flatten)]
    pub game_settings: GameSettingsInfo,
}

impl SeekInfo {
    pub fn from_seek_view(seek: SeekView) -> Self {
        SeekInfo {
            id: seek.id.0 as u64,
            creator_id: seek.creator_id.to_string(),
            opponent_id: seek.opponent_id.map(|id| id.to_string()),
            color: match seek.color {
                None => "random".to_string(),
                Some(TakPlayer::White) => "white".to_string(),
                Some(TakPlayer::Black) => "black".to_string(),
            },
            game_settings: GameSettingsInfo::from_game_settings(&seek.game_settings),
            is_rated: seek.is_rated,
        }
    }
}
