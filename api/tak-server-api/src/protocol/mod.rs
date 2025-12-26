mod v2;

use std::sync::Arc;

use tak_server_app::{
    Application,
    domain::{ListenerId, PlayerId},
};

use crate::{
    client::{ServerMessage, TransportServiceImpl},
    protocol::v2::ProtocolV2Handler,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Protocol {
    V0,
    V2,
}

impl Protocol {
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "0" => Some(Protocol::V0),
            "2" => Some(Protocol::V2),
            _ => None,
        }
    }
}

pub async fn handle_client_message(
    app: &Arc<Application>,
    transport: &TransportServiceImpl,
    protocol: &Protocol,
    id: ListenerId,
    msg: String,
) {
    match protocol {
        Protocol::V0 | Protocol::V2 => {
            ProtocolV2Handler::new(app, transport)
                .handle_client_message(id, msg)
                .await
        }
    }
}

pub async fn handle_server_message(
    app: &Arc<Application>,
    transport: &TransportServiceImpl,
    protocol: &Protocol,
    id: ListenerId,
    msg: &ServerMessage,
) {
    match protocol {
        Protocol::V0 | Protocol::V2 => {
            ProtocolV2Handler::new(app, transport)
                .handle_server_message(id, msg)
                .await
        }
    }
}

pub async fn on_authenticated(
    app: &Arc<Application>,
    transport: &TransportServiceImpl,
    protocol: &Protocol,
    id: ListenerId,
    player_id: PlayerId,
) {
    match protocol {
        Protocol::V0 | Protocol::V2 => {
            ProtocolV2Handler::new(app, transport)
                .on_authenticated(id, player_id)
                .await
        }
    }
}

pub fn on_connected(
    app: &Arc<Application>,
    transport: &TransportServiceImpl,
    protocol: &Protocol,
    id: ListenerId,
) {
    match protocol {
        Protocol::V0 | Protocol::V2 => ProtocolV2Handler::new(app, transport).on_connected(id),
    }
}

pub fn register_http_endpoints(
    router: axum::Router<Arc<Application>>,
) -> axum::Router<Arc<Application>> {
    //no HTTP endpoints yet
    router
}
