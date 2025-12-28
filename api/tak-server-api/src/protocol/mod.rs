mod v2;

use std::sync::Arc;

use tak_server_app::{
    Application,
    domain::{AccountId, ListenerId},
    ports::authentication::AuthenticationPort,
};

use crate::{
    acl::LegacyAPIAntiCorruptionLayer,
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

pub struct ProtocolService {
    handler_v2: Arc<ProtocolV2Handler>,
}

impl ProtocolService {
    pub fn new(
        app: Arc<Application>,
        transport: Arc<TransportServiceImpl>,
        auth: Arc<dyn AuthenticationPort + Send + Sync + 'static>,
        acl: Arc<LegacyAPIAntiCorruptionLayer>,
    ) -> Self {
        Self {
            handler_v2: Arc::new(ProtocolV2Handler::new(app, transport, auth, acl)),
        }
    }

    pub async fn handle_client_message(&self, protocol: &Protocol, id: ListenerId, msg: String) {
        match protocol {
            Protocol::V0 | Protocol::V2 => self.handler_v2.handle_client_message(id, msg).await,
        }
    }

    pub async fn handle_server_message(
        &self,
        protocol: &Protocol,
        id: ListenerId,
        msg: &ServerMessage,
    ) {
        match protocol {
            Protocol::V0 | Protocol::V2 => self.handler_v2.send_server_message(id, msg).await,
        }
    }

    pub async fn on_authenticated(
        &self,
        protocol: &Protocol,
        id: ListenerId,
        account_id: AccountId,
    ) {
        match protocol {
            Protocol::V0 | Protocol::V2 => self.handler_v2.on_authenticated(id, account_id).await,
        }
    }

    pub fn on_connected(&self, protocol: &Protocol, id: ListenerId) {
        match protocol {
            Protocol::V0 | Protocol::V2 => self.handler_v2.on_connected(id),
        }
    }
}

pub fn register_http_endpoints(
    router: axum::Router<Arc<Application>>,
) -> axum::Router<Arc<Application>> {
    //no HTTP endpoints yet
    router
}
