use std::time::Duration;

use axum::{Json, http::StatusCode};
use serde::Deserialize;

use crate::{
    jwt::Claims,
    player::{PlayerUsername, fetch_player},
    seek::{GameType, add_seek, remove_seek_of_player},
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
    Json(msg): Json<AddSeekMessage>,
) -> Result<(), (StatusCode, String)> {
    fetch_player(&claims.sub).ok_or((StatusCode::NOT_FOUND, "Player not found".to_string()))?;
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
    add_seek(
        claims.sub.to_string(),
        seek.opponent,
        color,
        game_settings,
        game_type,
    )
    .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(())
}

pub async fn handle_remove_seek_endpoint(claims: Claims) -> Result<(), (StatusCode, String)> {
    fetch_player(&claims.sub).ok_or((StatusCode::NOT_FOUND, "Player not found".to_string()))?;
    remove_seek_of_player(&claims.sub).map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;
    Ok(())
}
