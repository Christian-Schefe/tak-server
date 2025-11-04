use axum::{Json, extract::State};
use serde::Deserialize;
use tak_server_domain::{ServiceResult, app::AppState, player::PlayerUsername};

use crate::{
    app::MyServiceError,
    client::ClientId,
    protocol::json::{ClientResponse, ProtocolJsonHandler},
};

impl ProtocolJsonHandler {
    pub fn handle_login_message(
        &self,
        id: &ClientId,
        token: &str,
    ) -> ServiceResult<ClientResponse> {
        let username = self.player_service.try_login_jwt(&token)?;
        self.transport.associate_player(id, &username)?;
        Ok(ClientResponse::Ok)
    }

    pub fn handle_login_guest_message(
        &self,
        id: &ClientId,
        token: Option<&str>,
    ) -> ServiceResult<ClientResponse> {
        let username = self.player_service.try_login_guest(token)?;
        self.transport.associate_player(id, &username)?;
        Ok(ClientResponse::Ok)
    }
}

#[derive(Deserialize)]
pub struct RequestPasswordResetRequest {
    username: PlayerUsername,
    email: String,
}

pub async fn request_password_reset_endpoint(
    State(app): State<AppState>,
    Json(req): Json<RequestPasswordResetRequest>,
) -> Result<(), MyServiceError> {
    let username = req.username;
    let email = req.email;

    app.player_service.send_reset_token(&username, &email)?;

    Ok(())
}

#[derive(Deserialize)]
pub struct ResetPasswordRequest {
    username: PlayerUsername,
    token: String,
    new_password: String,
}

pub async fn reset_password_endpoint(
    State(app): State<AppState>,
    Json(req): Json<ResetPasswordRequest>,
) -> Result<(), MyServiceError> {
    let username = req.username;
    let token = req.token;
    let new_password = req.new_password;

    app.player_service
        .reset_password(&username, &token, &new_password)?;

    Ok(())
}

#[derive(Deserialize)]
pub struct ChangePasswordRequest {
    username: PlayerUsername,
    old_password: String,
    new_password: String,
}

pub async fn change_password_endpoint(
    State(app): State<AppState>,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<(), MyServiceError> {
    let username = req.username;
    let old_password = req.old_password;
    let new_password = req.new_password;

    app.player_service
        .change_password(&username, &old_password, &new_password)?;

    Ok(())
}
