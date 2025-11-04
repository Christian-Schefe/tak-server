mod json;
mod v2;

use tak_server_domain::{app::AppState, player::PlayerUsername, transport::ServerMessage};

use crate::{
    client::{ClientId, TransportServiceImpl},
    protocol::{json::ProtocolJsonHandler, v2::ProtocolV2Handler},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Protocol {
    V0,
    V2,
    JSON,
}

impl Protocol {
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "0" => Some(Protocol::V0),
            "2" => Some(Protocol::V2),
            "3" => Some(Protocol::JSON),
            _ => None,
        }
    }
}

pub fn handle_client_message(
    app_state: &AppState,
    transport: &TransportServiceImpl,
    protocol: &Protocol,
    id: &ClientId,
    msg: String,
) {
    match protocol {
        Protocol::V0 | Protocol::V2 => {
            ProtocolV2Handler::new(app_state, transport).handle_client_message(id, msg)
        }
        Protocol::JSON => {
            ProtocolJsonHandler::new(app_state, transport).handle_client_message(id, msg)
        }
    }
}

pub fn handle_server_message(
    app_state: &AppState,
    transport: &TransportServiceImpl,
    protocol: &Protocol,
    id: &ClientId,
    msg: &ServerMessage,
) {
    match protocol {
        Protocol::V0 | Protocol::V2 => {
            ProtocolV2Handler::new(app_state, transport).handle_server_message(id, msg)
        }
        Protocol::JSON => {
            ProtocolJsonHandler::new(app_state, transport).handle_server_message(id, msg)
        }
    }
}

pub fn on_authenticated(
    app_state: &AppState,
    transport: &TransportServiceImpl,
    protocol: &Protocol,
    id: &ClientId,
    username: &PlayerUsername,
) {
    match protocol {
        Protocol::V0 | Protocol::V2 => {
            ProtocolV2Handler::new(app_state, transport).on_authenticated(id, username)
        }
        Protocol::JSON => {}
    }
}

pub fn on_connected(
    app_state: &AppState,
    transport: &TransportServiceImpl,
    protocol: &Protocol,
    id: &ClientId,
) {
    match protocol {
        Protocol::V0 | Protocol::V2 => {
            ProtocolV2Handler::new(app_state, transport).on_connected(id)
        }
        Protocol::JSON => {}
    }
}

pub fn register_http_endpoints(router: axum::Router<AppState>) -> axum::Router<AppState> {
    router.nest("/v3", json::register_http_endpoints())
}
