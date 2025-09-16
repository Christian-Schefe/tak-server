use axum::{
    Router,
    routing::{delete, post},
};
use serde::{Deserialize, Serialize};

use crate::{
    client::{ClientId, send_to},
    protocol::ServerMessage,
};

mod auth;
mod seek;

#[derive(Clone, Debug, Deserialize)]
pub enum ClientMessage {
    Ping,
    Login { token: String },
    LoginGuest { token: Option<String> },
}

#[derive(Clone, Debug, Serialize)]
pub enum ClientResponse {
    Error { message: String },
    Ok,
}

pub fn handle_server_message(_id: &ClientId, msg: &ServerMessage) {
    match msg {
        _ => todo!(),
    }
}

pub fn register_http_endpoints() -> Router {
    Router::new()
        .route("/seek", post(seek::handle_add_seek_endpoint))
        .route("/seek", delete(seek::handle_remove_seek_endpoint))
}

pub fn send_json_to(id: &ClientId, msg: &impl serde::Serialize) {
    match serde_json::to_string(msg) {
        Ok(json) => send_to(id, &json),
        Err(e) => eprintln!(
            "Failed to serialize message to JSON for client {}: {}",
            id, e
        ),
    }
}

pub fn handle_client_message(id: &ClientId, msg: String) {
    let msg = match serde_json::from_str::<ClientMessage>(&msg) {
        Ok(msg) => msg,
        Err(e) => {
            println!("Failed to parse JSON message from client {}: {}", id, e);
            send_json_to(
                id,
                &ClientResponse::Error {
                    message: "Invalid JSON".to_string(),
                },
            );
            return;
        }
    };
    match msg {
        ClientMessage::Ping => {
            send_json_to(id, &ClientResponse::Ok);
        }
        ClientMessage::Login { token } => {
            auth::handle_login_message(id, &token);
        }
        ClientMessage::LoginGuest { token } => {
            auth::handle_login_guest_message(id, token.as_deref());
        }
    };
}
