use axum::{Json, extract::State};
use serde::Deserialize;

use crate::{AppState, ServiceError, jwt::Claims, player::PlayerUsername};

#[derive(Debug, Deserialize)]
pub struct SudoBanRequest {
    username: PlayerUsername,
    message: String,
}

pub async fn sudo_ban_endpoint(
    claims: Claims,
    State(app): State<AppState>,
    Json(req): Json<SudoBanRequest>,
) -> Result<(), ServiceError> {
    app.player_service.fetch_player(&claims.sub)?;
    app.player_service
        .set_banned(&claims.sub, &req.username, Some(req.message.clone()))?;
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct SudoUnbanRequest {
    username: PlayerUsername,
}

pub async fn sudo_unban_endpoint(
    claims: Claims,
    State(app): State<AppState>,
    Json(req): Json<SudoUnbanRequest>,
) -> Result<(), ServiceError> {
    app.player_service.fetch_player(&claims.sub)?;
    app.player_service
        .set_banned(&claims.sub, &req.username, None)?;
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct SudoSetRequest {
    username: PlayerUsername,
    set: bool,
}

pub async fn sudo_admin_endpoint(
    claims: Claims,
    State(app): State<AppState>,
    Json(req): Json<SudoSetRequest>,
) -> Result<(), ServiceError> {
    app.player_service.fetch_player(&claims.sub)?;
    app.player_service
        .set_admin(&claims.sub, &req.username, req.set)?;
    Ok(())
}

pub async fn sudo_mod_endpoint(
    claims: Claims,
    State(app): State<AppState>,
    Json(req): Json<SudoSetRequest>,
) -> Result<(), ServiceError> {
    app.player_service.fetch_player(&claims.sub)?;
    app.player_service
        .set_modded(&claims.sub, &req.username, req.set)?;
    Ok(())
}

pub async fn sudo_bot_endpoint(
    claims: Claims,
    State(app): State<AppState>,
    Json(req): Json<SudoSetRequest>,
) -> Result<(), ServiceError> {
    app.player_service.fetch_player(&claims.sub)?;
    app.player_service
        .set_bot(&claims.sub, &req.username, req.set)?;
    Ok(())
}

pub async fn sudo_gag_endpoint(
    claims: Claims,
    State(app): State<AppState>,
    Json(req): Json<SudoSetRequest>,
) -> Result<(), ServiceError> {
    app.player_service.fetch_player(&claims.sub)?;
    app.player_service
        .set_gagged(&claims.sub, &req.username, req.set)?;
    Ok(())
}
