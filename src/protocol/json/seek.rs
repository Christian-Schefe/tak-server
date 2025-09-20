use std::time::Duration;

use axum::{Json, extract::State};
use serde::Deserialize;

use crate::{
    AppState, ServiceError,
    jwt::Claims,
    player::PlayerUsername,
    seek::GameType,
    tak::{TakGameSettings, TakPlayer, TakTimeControl},
};

#[derive(Deserialize)]
pub struct AddSeekMessage {
    seek: JsonSeek,
}

#[derive(Clone, Debug, Deserialize)]
pub struct JsonSeek {
    pub opponent: Option<PlayerUsername>,
    pub color: SeekColor,
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

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SeekColor {
    Random,
    White,
    Black,
}

pub async fn handle_add_seek_endpoint(
    claims: Claims,
    State(app): State<AppState>,
    Json(msg): Json<AddSeekMessage>,
) -> Result<(), ServiceError> {
    app.player_service.fetch_player(&claims.sub)?;
    let seek = msg.seek;
    let game_settings = TakGameSettings {
        board_size: seek.board_size,
        half_komi: seek.half_komi,
        reserve_pieces: seek.reserve_pieces,
        reserve_capstones: seek.reserve_capstones,
        time_control: TakTimeControl {
            contingent: Duration::from_millis(seek.time_ms),
            increment: Duration::from_millis(seek.increment_ms),
            extra: match (seek.extra_move, seek.extra_time_ms) {
                (Some(moves), Some(time_ms)) if moves > 0 => {
                    Some((moves, Duration::from_millis(time_ms)))
                }
                _ => None,
            },
        },
    };
    let color = match seek.color {
        SeekColor::Random => None,
        SeekColor::White => Some(TakPlayer::White),
        SeekColor::Black => Some(TakPlayer::Black),
    };
    let game_type = if seek.tournament {
        GameType::Tournament
    } else if seek.unrated {
        GameType::Unrated
    } else {
        GameType::Rated
    };
    app.seek_service.add_seek(
        claims.sub.to_string(),
        seek.opponent,
        color,
        game_settings,
        game_type,
    )?;
    Ok(())
}

pub async fn handle_remove_seek_endpoint(
    claims: Claims,
    State(app): State<AppState>,
) -> Result<(), ServiceError> {
    app.player_service.fetch_player(&claims.sub)?;
    app.seek_service.remove_seek_of_player(&claims.sub)?;
    Ok(())
}
