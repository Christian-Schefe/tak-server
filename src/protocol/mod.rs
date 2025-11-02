mod json;
mod v2;

use tak_server_domain::{player::PlayerUsername, transport::ServerMessage};

use crate::{AppState, client::ClientId};

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

pub type ArcProtocolService = std::sync::Arc<dyn ProtocolService + Send + Sync>;
pub trait ProtocolService {
    fn init(&self, client_service: &AppState);
    fn handle_client_message(&self, protocol: &Protocol, id: &ClientId, msg: String);
    fn handle_server_message(&self, protocol: &Protocol, id: &ClientId, msg: &ServerMessage);
    fn on_authenticated(&self, protocol: &Protocol, id: &ClientId, username: &PlayerUsername);
    fn on_connected(&self, protocol: &Protocol, id: &ClientId);
    fn register_http_endpoints(&self, router: axum::Router<AppState>) -> axum::Router<AppState>;
}

pub struct ProtocolServiceImpl {
    v2: LazyInit<v2::ProtocolV2Handler>,
    json: LazyInit<json::ProtocolJsonHandler>,
}

impl ProtocolServiceImpl {
    pub fn new() -> Self {
        Self {
            v2: LazyInit::new(),
            json: LazyInit::new(),
        }
    }
}

impl ProtocolService for ProtocolServiceImpl {
    fn init(&self, app: &AppState) {
        let _ = self.v2.init(v2::ProtocolV2Handler::new(
            app.client_service.clone(),
            app.seek_service.clone(),
            app.player_service.clone(),
            app.chat_service.clone(),
            app.game_service.clone(),
        ));
        let _ = self.json.init(json::ProtocolJsonHandler::new(
            app.client_service.clone(),
            app.player_service.clone(),
            app.game_service.clone(),
            app.chat_service.clone(),
        ));
    }

    fn handle_client_message(&self, protocol: &Protocol, id: &ClientId, msg: String) {
        match protocol {
            Protocol::V0 | Protocol::V2 => self.v2.get().handle_client_message(id, msg),
            Protocol::JSON => self.json.get().handle_client_message(id, msg),
        }
    }

    fn handle_server_message(&self, protocol: &Protocol, id: &ClientId, msg: &ServerMessage) {
        match protocol {
            Protocol::V0 | Protocol::V2 => self.v2.get().handle_server_message(id, msg),
            Protocol::JSON => self.json.get().handle_server_message(id, msg),
        }
    }

    fn on_authenticated(&self, protocol: &Protocol, id: &ClientId, username: &PlayerUsername) {
        match protocol {
            Protocol::V0 | Protocol::V2 => self.v2.get().on_authenticated(id, username),
            Protocol::JSON => {}
        }
    }

    fn on_connected(&self, protocol: &Protocol, id: &ClientId) {
        match protocol {
            Protocol::V0 | Protocol::V2 => self.v2.get().on_connected(id),
            Protocol::JSON => {}
        }
    }

    fn register_http_endpoints(&self, router: axum::Router<AppState>) -> axum::Router<AppState> {
        router.nest("/v3", json::register_http_endpoints())
    }
}
