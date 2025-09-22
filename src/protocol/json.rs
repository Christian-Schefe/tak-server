use axum::{
    Router,
    routing::{delete, post},
};
use serde::{Deserialize, Serialize};

use crate::{
    AppState, ArcClientService, ArcPlayerService, ServiceResult, client::ClientId,
    protocol::ServerMessage,
};

mod auth;
mod seek;

pub struct ProtocolJsonHandler {
    client_service: ArcClientService,
    player_service: ArcPlayerService,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ClientMessage {
    Ping,
    Login { token: String },
    LoginGuest { token: Option<String> },
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackedClientMessage {
    pub msg_id: Option<String>,
    #[serde(flatten)]
    pub msg: ClientMessage,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ClientResponse {
    Error { message: String },
    Ok,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackedClientResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msg_id: Option<String>,
    #[serde(flatten)]
    pub response: ClientResponse,
}

impl ProtocolJsonHandler {
    pub fn new(client_service: ArcClientService, player_service: ArcPlayerService) -> Self {
        Self {
            client_service,
            player_service,
        }
    }

    pub fn handle_server_message(&self, _id: &ClientId, msg: &ServerMessage) {
        match msg {
            _ => {}
        }
    }

    pub fn send_json_to(&self, id: &ClientId, msg: &impl serde::Serialize) {
        match serde_json::to_string(msg) {
            Ok(json) => crate::client::send_to(&**self.client_service, id, &json),
            Err(e) => eprintln!(
                "Failed to serialize message to JSON for client {}: {}",
                id, e
            ),
        }
    }

    pub fn handle_client_message(&self, id: &ClientId, msg: String) {
        if msg.to_ascii_lowercase().starts_with("protocol") {
            return;
        }
        let msg = match serde_json::from_str::<TrackedClientMessage>(&msg) {
            Ok(msg) => msg,
            Err(e) => {
                println!("Failed to parse JSON message from client {}: {}", id, e);
                self.send_json_to(
                    id,
                    &ClientResponse::Error {
                        message: "Invalid JSON".to_string(),
                    },
                );
                return;
            }
        };
        let response: ServiceResult<ClientResponse> = match msg.msg {
            ClientMessage::Ping => Ok(ClientResponse::Ok),
            ClientMessage::Login { token } => self.handle_login_message(id, &token),
            ClientMessage::LoginGuest { token } => {
                self.handle_login_guest_message(id, token.as_deref())
            }
        };
        let tracked_response = TrackedClientResponse {
            msg_id: msg.msg_id,
            response: response.unwrap_or_else(|e| ClientResponse::Error {
                message: e.to_string(),
            }),
        };
        self.send_json_to(id, &tracked_response);
    }
}

pub fn register_http_endpoints() -> Router<AppState> {
    Router::new()
        .route("/seek", post(seek::handle_add_seek_endpoint))
        .route("/seek", delete(seek::handle_remove_seek_endpoint))
}
