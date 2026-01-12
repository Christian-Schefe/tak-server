use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use more_concurrent_maps::bijection::BiMap;
use parking_lot::RwLock;
use tak_server_app::{
    Application,
    domain::{AccountId, ListenerId},
    ports::{
        connection::AccountConnectionPort,
        notification::{ListenerMessage, ListenerNotificationPort},
    },
};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ConnectionId(Uuid);

impl ConnectionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl std::fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub trait PlayerSimpleConnectionPort {
    fn notify_connection(&self, connection_id: ConnectionId, message: &ListenerMessage);
}

struct ConnectionRegistry {
    listener_map: BiMap<AccountId, ListenerId>,
    connection_to_listener: HashMap<ConnectionId, ListenerId>,
    listener_to_connections: HashMap<ListenerId, HashSet<ConnectionId>>,
}

pub struct PlayerConnectionDriver {
    app: Arc<Application>,
    inner: Arc<PlayerNotificationService>,
}

impl PlayerConnectionDriver {
    pub fn new(app: Arc<Application>, inner: Arc<PlayerNotificationService>) -> Self {
        Self { app, inner }
    }

    pub async fn add_connection(
        &self,
        account_id: &AccountId,
        connection_id: ConnectionId,
    ) -> bool {
        let mut registry = self.inner.registry.write();
        if registry.connection_to_listener.contains_key(&connection_id) {
            return false;
        }
        let listener_id = if let Some(listener_id) = registry.listener_map.get_by_left(&account_id)
        {
            *listener_id
        } else {
            let new_listener_id = ListenerId::new();
            registry
                .listener_map
                .try_insert(account_id.clone(), new_listener_id);
            //Set account online
            self.app.account_set_online_use_case.set_online(account_id);
            new_listener_id
        };
        registry
            .connection_to_listener
            .insert(connection_id, listener_id);
        registry
            .listener_to_connections
            .entry(listener_id)
            .or_insert_with(HashSet::new)
            .insert(connection_id);
        true
    }

    pub async fn remove_connection(&self, connection_id: &ConnectionId) {
        let mut registry = self.inner.registry.write();
        if let Some(listener_id) = registry.connection_to_listener.remove(connection_id) {
            if let Some(connections) = registry.listener_to_connections.get_mut(&listener_id) {
                connections.remove(connection_id);
                if connections.is_empty() {
                    registry.listener_to_connections.remove(&listener_id);
                    if let Some(account_id) = registry.listener_map.remove_by_right(&listener_id) {
                        self.set_player_offline(&account_id).await;
                    }
                }
            }
        }
    }

    pub fn get_account_id(&self, connection_id: &ConnectionId) -> Option<AccountId> {
        let registry = self.inner.registry.read();
        if let Some(listener_id) = registry.connection_to_listener.get(connection_id) {
            if let Some(account_id) = registry.listener_map.get_by_right(listener_id) {
                return Some(account_id.clone());
            }
        }
        None
    }

    async fn set_player_offline(&self, account_id: &AccountId) {
        self.app.account_set_online_use_case.set_offline(account_id);
        if let Ok(player_id) = self
            .app
            .player_resolver_service
            .resolve_player_id_by_account_id(account_id)
            .await
        {
            self.app.seek_cancel_use_case.cancel_seek(player_id);
        }
    }
}

#[async_trait::async_trait]
impl AccountConnectionPort for PlayerNotificationService {
    async fn get_connection_id(&self, account_id: &AccountId) -> Option<ListenerId> {
        let registry = self.registry.read();

        registry.listener_map.get_by_left(account_id).cloned()
    }
}

pub struct PlayerNotificationService {
    services: Vec<Arc<dyn PlayerSimpleConnectionPort + Send + Sync>>,
    registry: Arc<RwLock<ConnectionRegistry>>,
}

impl PlayerNotificationService {
    pub fn new(services: Vec<Arc<dyn PlayerSimpleConnectionPort + Send + Sync>>) -> Self {
        Self {
            services,
            registry: Arc::new(RwLock::new(ConnectionRegistry {
                listener_map: BiMap::new(),
                connection_to_listener: HashMap::new(),
                listener_to_connections: HashMap::new(),
            })),
        }
    }
}

impl ListenerNotificationPort for PlayerNotificationService {
    fn notify_listener(&self, listener: ListenerId, message: ListenerMessage) {
        let registry = self.registry.read();
        if let Some(connections) = registry.listener_to_connections.get(&listener) {
            for connection_id in connections {
                for service in &self.services {
                    service.notify_connection(*connection_id, &message);
                }
            }
        }
    }

    fn notify_listeners(&self, listeners: &[ListenerId], message: ListenerMessage) {
        let registry = self.registry.read();
        for listener in listeners {
            if let Some(connections) = registry.listener_to_connections.get(listener) {
                for connection_id in connections {
                    for service in &self.services {
                        service.notify_connection(*connection_id, &message);
                    }
                }
            }
        }
    }

    fn notify_all(&self, message: ListenerMessage) {
        let registry = self.registry.read();
        for connections in registry.listener_to_connections.values() {
            for connection_id in connections {
                for service in &self.services {
                    service.notify_connection(*connection_id, &message);
                }
            }
        }
    }
}
