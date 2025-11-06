use std::time::Duration;

use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use tak_server_domain::{
    app::AppState,
    game::GameType,
    player::PlayerUsername,
    seek::{Seek, SeekId},
};

use crate::{app::MyServiceError, jwt::Claims};
use tak_core::{TakGameSettings, TakPlayer, TakTimeControl};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonSeekWithId {
    pub id: SeekId,
    #[serde(flatten)]
    pub seek: JsonSeek,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SeekColor {
    Random,
    White,
    Black,
}

fn json_seek_from_seek(seek: &Seek) -> JsonSeek {
    JsonSeek {
        opponent: seek.opponent.clone(),
        color: match seek.color {
            None => SeekColor::Random,
            Some(TakPlayer::White) => SeekColor::White,
            Some(TakPlayer::Black) => SeekColor::Black,
        },
        tournament: matches!(seek.game_type, GameType::Tournament),
        unrated: matches!(seek.game_type, GameType::Unrated),
        board_size: seek.game_settings.board_size,
        half_komi: seek.game_settings.half_komi,
        reserve_pieces: seek.game_settings.reserve_pieces,
        reserve_capstones: seek.game_settings.reserve_capstones,
        time_ms: seek.game_settings.time_control.contingent.as_millis() as u64,
        increment_ms: seek.game_settings.time_control.increment.as_millis() as u64,
        extra_move: seek
            .game_settings
            .time_control
            .extra
            .map(|(moves, _)| moves),
        extra_time_ms: seek
            .game_settings
            .time_control
            .extra
            .map(|(_, time)| time.as_millis() as u64),
    }
}

pub async fn handle_add_seek_endpoint(
    claims: Claims,
    State(app): State<AppState>,
    Json(seek): Json<JsonSeek>,
) -> Result<(), MyServiceError> {
    app.player_service.fetch_player(&claims.sub).await?;
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
) -> Result<(), MyServiceError> {
    app.player_service.fetch_player(&claims.sub).await?;
    app.seek_service.remove_seek_of_player(&claims.sub)?;
    Ok(())
}

pub async fn get_seeks_endpoint(
    claims: Claims,
    State(app): State<AppState>,
) -> Result<Json<Vec<JsonSeekWithId>>, MyServiceError> {
    app.player_service.fetch_player(&claims.sub).await?;
    let seeks = app.seek_service.get_seeks();
    let json_seeks = seeks
        .into_iter()
        .map(|seek| JsonSeekWithId {
            id: seek.id,
            seek: json_seek_from_seek(&seek),
        })
        .collect();
    Ok(Json(json_seeks))
}

pub async fn get_seek_endpoint(
    claims: Claims,
    Path(seek_id): Path<SeekId>,
    State(app): State<AppState>,
) -> Result<Json<JsonSeek>, MyServiceError> {
    app.player_service.fetch_player(&claims.sub).await?;
    let seek = app.seek_service.get_seek(&seek_id)?;
    let json_seek = json_seek_from_seek(&seek);
    Ok(Json(json_seek))
}

pub async fn accept_seek_endpoint(
    claims: Claims,
    Path(seek_id): Path<SeekId>,
    State(app): State<AppState>,
) -> Result<(), MyServiceError> {
    app.player_service.fetch_player(&claims.sub).await?;
    app.seek_service.accept_seek(&claims.sub, &seek_id).await?;
    Ok(())
}
