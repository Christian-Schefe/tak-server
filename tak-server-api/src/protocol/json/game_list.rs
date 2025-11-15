use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use tak_server_domain::{
    ServiceError, ServiceResult,
    app::AppState,
    game::{GameId, GameType},
    player::PlayerUsername,
    transport::ListenerId,
};

use crate::{
    app::MyServiceError,
    jwt::Claims,
    protocol::json::{ClientResponse, ProtocolJsonHandler},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct JsonGame {
    pub white: PlayerUsername,
    pub black: PlayerUsername,
    pub tournament: bool,
    pub unrated: bool,
    pub board_size: u32,
    pub half_komi: u32,
    pub reserve_pieces: u32,
    pub reserve_capstones: u32,
    pub time_ms: u64,
    pub increment_ms: u64,
    pub extra_move: Option<u32>,
    pub extra_time_ms: Option<u64>,
}

pub async fn get_game_ids_endpoint(
    claims: Claims,
    State(app): State<AppState>,
) -> Result<Json<Vec<GameId>>, MyServiceError> {
    app.player_service.fetch_player(&claims.sub).await?;
    let game_ids = app.game_service.get_game_ids();
    Ok(Json(game_ids))
}

pub async fn get_game_endpoint(
    claims: Claims,
    Path(game_id): Path<GameId>,
    State(app): State<AppState>,
) -> Result<Json<JsonGame>, MyServiceError> {
    app.player_service.fetch_player(&claims.sub).await?;
    let Some(game) = app.game_service.get_game(&game_id) else {
        return ServiceError::not_found("Game ID not found")?;
    };
    let settings = &game.game.base.settings;
    let json_game = JsonGame {
        white: game.white,
        black: game.black,
        tournament: matches!(game.game_type, GameType::Tournament),
        unrated: matches!(game.game_type, GameType::Unrated),
        board_size: settings.board_size,
        half_komi: settings.half_komi,
        reserve_pieces: settings.reserve.pieces,
        reserve_capstones: settings.reserve.capstones,
        time_ms: settings.time_control.contingent.as_millis() as u64,
        increment_ms: settings.time_control.increment.as_millis() as u64,
        extra_move: settings.time_control.extra.map(|(moves, _)| moves),
        extra_time_ms: settings
            .time_control
            .extra
            .map(|(_, time)| time.as_millis() as u64),
    };
    Ok(Json(json_game))
}

impl ProtocolJsonHandler {
    pub fn handle_observe_game_message(
        &self,
        id: ListenerId,
        game_id: &GameId,
        observe: bool,
    ) -> ServiceResult<ClientResponse> {
        if observe {
            self.game_service.observe_game(id, game_id)?;
            Ok(ClientResponse::Ok)
        } else {
            self.game_service.unobserve_game(id, game_id)?;
            Ok(ClientResponse::Ok)
        }
    }
}
