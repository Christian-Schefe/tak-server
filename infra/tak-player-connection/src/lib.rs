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
        connection::{AccountConnectionPort, AccountOnlineStatusPort},
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
    inner: Arc<PlayerConnectionService>,
}

impl PlayerConnectionDriver {
    pub fn new(app: Arc<Application>, inner: Arc<PlayerConnectionService>) -> Self {
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
        let (listener_id, set_online) =
            if let Some(listener_id) = registry.listener_map.get_by_left(&account_id) {
                (*listener_id, false)
            } else {
                let new_listener_id = ListenerId::new();
                registry
                    .listener_map
                    .try_insert(account_id.clone(), new_listener_id);
                (new_listener_id, true)
            };
        registry
            .connection_to_listener
            .insert(connection_id, listener_id);
        registry
            .listener_to_connections
            .entry(listener_id)
            .or_insert_with(HashSet::new)
            .insert(connection_id);
        if set_online {
            drop(registry);
            self.app.account_set_online_use_case.set_online(account_id);
        }
        true
    }

    pub async fn remove_connection(&self, connection_id: &ConnectionId) {
        let mut set_account_offline = None;
        {
            let mut registry = self.inner.registry.write();
            if let Some(listener_id) = registry.connection_to_listener.remove(connection_id) {
                if let Some(connections) = registry.listener_to_connections.get_mut(&listener_id) {
                    connections.remove(connection_id);
                    if connections.is_empty() {
                        registry.listener_to_connections.remove(&listener_id);
                        if let Some(account_id) =
                            registry.listener_map.remove_by_right(&listener_id)
                        {
                            set_account_offline = Some(account_id);
                        }
                    }
                }
            }
        };
        if let Some(account_id) = set_account_offline {
            self.set_player_offline(&account_id).await;
        }
    }

    pub fn get_connection_ids(&self, account_id: &AccountId) -> Vec<ConnectionId> {
        let registry = self.inner.registry.read();
        if let Some(listener_id) = registry.listener_map.get_by_left(account_id) {
            if let Some(conn_set) = registry.listener_to_connections.get(listener_id) {
                return conn_set.iter().cloned().collect();
            }
        }
        Vec::new()
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

    pub fn get_listener_id(&self, account_id: &AccountId) -> Option<ListenerId> {
        let registry = self.inner.registry.read();
        registry.listener_map.get_by_left(account_id).cloned()
    }

    async fn set_player_offline(&self, account_id: &AccountId) {
        self.app.account_set_online_use_case.set_offline(account_id);
        if let Ok(player_id) = self
            .app
            .player_resolver_service
            .resolve_player_id_by_account_id(account_id)
            .await
        {
            self.app.seek_cancel_use_case.cancel_seeks(player_id);
        }
    }
}

#[async_trait::async_trait]
impl AccountConnectionPort for PlayerConnectionService {
    async fn get_connection_id(&self, account_id: &AccountId) -> Option<ListenerId> {
        let registry = self.registry.read();

        registry.listener_map.get_by_left(account_id).cloned()
    }
}

pub struct PlayerConnectionService {
    services: Vec<Arc<dyn PlayerSimpleConnectionPort + Send + Sync>>,
    registry: Arc<RwLock<ConnectionRegistry>>,
}

impl PlayerConnectionService {
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

impl ListenerNotificationPort for PlayerConnectionService {
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

pub struct AccountOnlineStatusService {
    online_accounts: Arc<RwLock<HashSet<AccountId>>>,
}

impl AccountOnlineStatusService {
    pub fn new() -> Self {
        Self {
            online_accounts: Arc::new(RwLock::new(HashSet::new())),
        }
    }
}

impl AccountOnlineStatusPort for AccountOnlineStatusService {
    fn set_account_online(&self, account_id: &AccountId) -> Option<Vec<AccountId>> {
        let mut online_accounts = self.online_accounts.write();
        if online_accounts.insert(account_id.clone()) {
            Some(online_accounts.iter().cloned().collect())
        } else {
            None
        }
    }

    fn set_account_offline(&self, account_id: &AccountId) -> Option<Vec<AccountId>> {
        let mut online_accounts = self.online_accounts.write();
        if online_accounts.remove(account_id) {
            Some(online_accounts.iter().cloned().collect())
        } else {
            None
        }
    }

    fn get_online_accounts(&self) -> Vec<AccountId> {
        self.online_accounts.read().iter().cloned().collect()
    }
}
