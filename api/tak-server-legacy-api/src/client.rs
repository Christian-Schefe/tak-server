use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};

use axum::{
    Router,
    extract::{
        WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::Response,
    routing::any,
};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};

use parking_lot::RwLock;
use tak_server_app::{
    Application,
    domain::{AccountId, ListenerId, PlayerId},
    ports::{
        authentication::AuthenticationPort,
        connection::PlayerConnectionPort,
        notification::{ListenerMessage, ListenerNotificationPort},
    },
};
use tokio::{
    net::TcpStream,
    select,
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
};
use tokio_util::{
    codec::{Framed, LinesCodec},
    sync::CancellationToken,
};
use uuid::Uuid;

use crate::{
    acl::LegacyAPIAntiCorruptionLayer,
    protocol::{Protocol, ProtocolService},
};

pub enum ClientMessage {
    Text(String),
    Close,
}

#[derive(Clone, Debug)]
pub enum ServerMessage {
    Notification(ListenerMessage),
    ConnectionClosed { reason: DisconnectReason },
}

static APPLICATION: OnceLock<Arc<Application>> = OnceLock::new();
static PROTOCOL_SERVICE: OnceLock<Arc<ProtocolService>> = OnceLock::new();

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

pub struct ClientConnection {
    id: ConnectionId,
    sender: UnboundedSender<String>,
    cancellation_token: CancellationToken,
    last_activity: Instant,
    protocol: Protocol,
}

pub struct Client {
    server_message_handler: UnboundedSender<ServerMessage>,
    connections: HashSet<ConnectionId>,
    account_id: AccountId,
}

pub struct ClientRegistry {
    clients: HashMap<ListenerId, Client>,
    connection_to_listener: HashMap<ConnectionId, ListenerId>,
    account_to_listener: HashMap<AccountId, ListenerId>,
    connections: HashMap<ConnectionId, ClientConnection>,
}

impl ClientRegistry {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
            account_to_listener: HashMap::new(),
            connection_to_listener: HashMap::new(),
            connections: HashMap::new(),
        }
    }

    fn refresh_last_activity(&mut self, connection_id: &ConnectionId) {
        if let Some(conn) = self.connections.get_mut(connection_id) {
            conn.last_activity = Instant::now();
        }
    }

    fn set_protocol(&mut self, connection_id: &ConnectionId, protocol: Protocol) {
        if let Some(conn) = self.connections.get_mut(connection_id) {
            conn.protocol = protocol;
        }
    }

    fn get_listener(&self, account_id: &AccountId) -> Option<&ListenerId> {
        self.account_to_listener.get(account_id)
    }

    fn get_connection(&self, connection_id: &ConnectionId) -> Option<&ClientConnection> {
        self.connections.get(connection_id)
    }

    fn get_connections(&self) -> &HashMap<ConnectionId, ClientConnection> {
        &self.connections
    }

    fn get_client(&self, listener_id: &ListenerId) -> Option<&Client> {
        self.clients.get(listener_id)
    }

    fn get_client_by_connection(&self, connection_id: &ConnectionId) -> Option<&Client> {
        let listener_id = self.connection_to_listener.get(connection_id)?;
        self.clients.get(listener_id)
    }

    fn get_clients(&self) -> &HashMap<ListenerId, Client> {
        &self.clients
    }

    fn open_connection(&mut self, connection: ClientConnection) {
        self.connections.insert(connection.id, connection);
    }

    fn close_connection(&mut self, connection_id: &ConnectionId) -> Option<AccountId> {
        if let Some(conn) = self.connections.remove(connection_id) {
            conn.cancellation_token.cancel();
        }
        if let Some(listener_id) = self.connection_to_listener.remove(connection_id) {
            if let Some(client) = self.clients.get_mut(&listener_id) {
                client.connections.remove(connection_id);
                let account_id = client.account_id.clone();
                if client.connections.is_empty() {
                    self.clients.remove(&listener_id);
                    self.account_to_listener.remove(&account_id);
                    return Some(account_id);
                }
            }
        }
        None
    }

    fn associate_connection_with_account(
        &mut self,
        connection_id: ConnectionId,
        account_id: AccountId,
        create_client: impl FnOnce(AccountId, ListenerId) -> Client,
    ) -> bool {
        if self.connection_to_listener.contains_key(&connection_id) {
            return false;
        }
        let listener_id = *self
            .account_to_listener
            .entry(account_id.clone())
            .or_insert_with(|| ListenerId::new());
        self.connection_to_listener
            .insert(connection_id, listener_id);
        let client = self
            .clients
            .entry(listener_id)
            .or_insert_with(move || create_client(account_id, listener_id));
        client.connections.insert(connection_id);
        true
    }
}

#[derive(Clone)]
pub struct TransportServiceImpl {
    client_registry: Arc<RwLock<ClientRegistry>>,
}

impl TransportServiceImpl {
    pub fn new() -> Self {
        Self {
            client_registry: Arc::new(RwLock::new(ClientRegistry::new())),
        }
    }

    async fn on_disconnect(&self, id: ConnectionId) {
        let maybe_disconnected_account_id = self.client_registry.write().close_connection(&id);
        if let Some(account_id) = maybe_disconnected_account_id {
            let app = APPLICATION.get().unwrap();
            if let Ok(player_id) = app
                .player_resolver_service
                .resolve_player_id_by_account_id(&account_id)
                .await
            {
                app.player_set_online_use_case.set_offline(player_id);
                app.seek_cancel_use_case.cancel_seek(player_id);
            }
        }
    }

    async fn handle_connection<M, S, E>(
        &self,
        socket: S,
        msg_factory: impl Fn(String) -> M + Send + 'static,
        msg_parser: impl Fn(M) -> Option<ClientMessage> + Send + 'static,
    ) where
        S: futures_util::Sink<M>
            + futures_util::Stream<Item = Result<M, E>>
            + Unpin
            + Send
            + 'static,
        M: Send + 'static,
        E: Send + 'static,
    {
        let (ws_sender, ws_receiver) = socket.split();
        let cancellation_token = CancellationToken::new();

        let client_service = self.clone();
        let cancellation_token_clone = cancellation_token.clone();
        let connection_id = ConnectionId::new();

        let receive_task = tokio::spawn(async move {
            client_service
                .handle_receive::<S, M, E>(
                    connection_id,
                    ws_receiver,
                    cancellation_token_clone,
                    msg_parser,
                )
                .await;
        });

        let client_service = self.clone();
        let cancellation_token_clone = cancellation_token.clone();
        let (send_tx, send_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

        let send_task = tokio::spawn(async move {
            client_service
                .handle_send::<S, M>(
                    connection_id,
                    ws_sender,
                    cancellation_token_clone,
                    msg_factory,
                    send_rx,
                )
                .await;
        });

        let connection = ClientConnection {
            id: connection_id,
            sender: send_tx,
            cancellation_token: cancellation_token.clone(),
            last_activity: Instant::now(),
            protocol: Protocol::V0,
        };

        self.client_registry.write().open_connection(connection);

        let _ = tokio::join!(receive_task, send_task);
        self.on_disconnect(connection_id).await;
        log::info!("Client {} fully disconnected", connection_id);
    }

    fn create_client(
        &self,
        account_id: AccountId,
        client_id: ListenerId,
        cancellation_token: CancellationToken,
    ) -> Client {
        let (notification_tx, notification_rx) =
            tokio::sync::mpsc::unbounded_channel::<ServerMessage>();

        let client_service = self.clone();
        let cancellation_token_clone = cancellation_token.clone();
        tokio::spawn(async move {
            client_service
                .handle_notification(notification_rx, cancellation_token_clone, client_id)
                .await;
        });

        let client = Client {
            server_message_handler: notification_tx,
            connections: HashSet::new(),
            account_id,
        };
        client
    }

    async fn handle_notification(
        &self,
        mut rx: UnboundedReceiver<ServerMessage>,
        cancellation_token: CancellationToken,
        id: ListenerId,
    ) {
        while let Some(msg) = select! {
            msg = rx.recv() => msg,
            _ = cancellation_token.cancelled() => None,
        } {
            let protocol_conn_id_pairs = {
                let registry = self.client_registry.read();
                if let Some(client) = registry.get_client(&id) {
                    Some(
                        client
                            .connections
                            .iter()
                            .filter_map(|conn_id| {
                                registry
                                    .get_connection(conn_id)
                                    .map(|conn| (conn.protocol.clone(), *conn_id))
                            })
                            .collect::<Vec<_>>(),
                    )
                } else {
                    None
                }
            };
            if let Some(protocol_conn_id_pairs) = protocol_conn_id_pairs {
                for (protocol, conn_id) in &protocol_conn_id_pairs {
                    PROTOCOL_SERVICE
                        .get()
                        .unwrap()
                        .handle_server_message(&protocol, *conn_id, &msg)
                        .await;
                }
            }
        }
        log::info!("Client {} notification ended", id);
    }

    async fn handle_send<S, M>(
        &self,
        id: ConnectionId,
        mut ws_sender: impl SinkExt<M> + Unpin + Send + 'static,
        cancellation_token: CancellationToken,
        msg_factory: impl Fn(String) -> M + Send + 'static,
        mut rx: UnboundedReceiver<String>,
    ) where
        S: futures_util::Sink<M> + Unpin + Send + 'static,
        M: Send + 'static,
    {
        self.on_connect(id);

        while let Some(msg) = select! {
            msg = rx.recv() => msg,
            _ = cancellation_token.cancelled() => None,
        } {
            let msg = msg_factory(msg);
            if ws_sender.send(msg).await.is_err() {
                break;
            }
        }
        let _ = ws_sender.close().await;
        log::info!("Client {} send ended", id);
        cancellation_token.cancel();
    }

    async fn handle_receive<S, M, E>(
        &self,
        id: ConnectionId,
        mut ws_receiver: impl StreamExt<Item = Result<M, E>> + Unpin + Send + 'static,
        cancellation_token: CancellationToken,
        msg_parser: impl Fn(M) -> Option<ClientMessage> + Send + 'static,
    ) where
        S: futures_util::Stream<Item = Result<M, E>> + Unpin + Send + 'static,
        M: Send + 'static,
        E: Send + 'static,
    {
        while let Some(Ok(msg)) = select! {
            msg = ws_receiver.next() => msg,
            _ = cancellation_token.cancelled() => None,
        } {
            {
                let mut registry = self.client_registry.write();
                registry.refresh_last_activity(&id);
            }

            let msg = match msg_parser(msg) {
                Some(m) => m,
                None => {
                    log::info!("Client {} sent invalid message", id);
                    continue;
                }
            };
            match msg {
                ClientMessage::Text(text) => {
                    if text.to_ascii_lowercase().starts_with("protocol") {
                        self.try_switch_protocol(id, &text).unwrap_or_else(|e| {
                            log::error!(
                                "Client {} failed to switch to protocol {}: {}",
                                id,
                                text,
                                e
                            );
                        });
                        // message is still passed to handler to allow protocol to respond.
                    }

                    let protocol = {
                        let registry = self.client_registry.read();
                        if let Some(conn) = registry.get_connection(&id) {
                            conn.protocol.clone()
                        } else {
                            log::error!("Client {} connection not found", id);
                            continue;
                        }
                    };

                    PROTOCOL_SERVICE
                        .get()
                        .unwrap()
                        .handle_client_message(&protocol, id, text)
                        .await;
                }
                ClientMessage::Close => break,
            }
        }
        log::info!("Client {} received ended", id);
        cancellation_token.cancel();
    }

    fn try_switch_protocol(&self, id: ConnectionId, protocol_msg: &str) -> Result<(), String> {
        let parts: Vec<&str> = protocol_msg.split_whitespace().collect();
        if parts.len() == 2 {
            let protocol = Protocol::from_id(parts[1]).ok_or("Unknown protocol")?;
            self.client_registry.write().set_protocol(&id, protocol);
            Ok(())
        } else {
            Err("Invalid protocol message format".into())
        }
    }

    fn on_connect(&self, id: ConnectionId) {
        log::info!("Client {} connected", id);
        if let Some(conn) = self.client_registry.read().get_connection(&id) {
            let protocol = conn.protocol.clone();
            PROTOCOL_SERVICE.get().unwrap().on_connected(&protocol, id);
        }
    }

    pub fn try_send_to(&self, id: ConnectionId, msg: &str) -> Result<(), String> {
        if let Some(conn) = self.client_registry.read().get_connection(&id) {
            conn.sender
                .send(msg.into())
                .map_err(|e| format!("Failed to send message: {}", e))
        } else {
            Err("Sender not initialized".into())
        }
    }

    pub async fn associate_account(
        &self,
        id: ConnectionId,
        account_id: AccountId,
    ) -> Result<(), String> {
        if !self
            .client_registry
            .write()
            .associate_connection_with_account(id, account_id.clone(), |account_id, listener_id| {
                self.create_client(account_id, listener_id, CancellationToken::new())
            })
        {
            return Err("Already logged in".into());
        }

        if let Some(protocol) = {
            self.client_registry
                .read()
                .get_connection(&id)
                .map(|c| c.protocol.clone())
        } {
            PROTOCOL_SERVICE
                .get()
                .unwrap()
                .on_authenticated(&protocol, id, &account_id)
                .await;
        }

        let app = APPLICATION.get().unwrap();
        if let Ok(player_id) = app
            .player_resolver_service
            .resolve_player_id_by_account_id(&account_id)
            .await
        {
            app.player_set_online_use_case.set_online(player_id);
        }
        Ok(())
    }

    pub async fn get_associated_player_and_account(
        &self,
        id: ConnectionId,
    ) -> Option<(PlayerId, AccountId)> {
        let account_id = {
            let registry = self.client_registry.read();
            let client = registry.get_client_by_connection(&id)?;
            client.account_id.clone()
        };
        let app = APPLICATION.get().unwrap();
        let player_id = app
            .player_resolver_service
            .resolve_player_id_by_account_id(&account_id)
            .await
            .ok()?;
        Some((player_id, account_id))
    }

    pub async fn close_connections_with_reason(&self, id: ListenerId, reason: DisconnectReason) {
        let connection_ids = {
            let registry = self.client_registry.read();
            if let Some(client) = registry.get_client(&id) {
                client.connections.iter().cloned().collect::<Vec<_>>()
            } else {
                vec![]
            }
        };

        for connection_id in connection_ids {
            self.close_with_reason(connection_id, reason.clone()).await;
        }
    }

    pub async fn close_with_reason(&self, id: ConnectionId, reason: DisconnectReason) {
        let (protocol, cancellation_token) = {
            let registry = self.client_registry.read();
            if let Some(conn) = registry.get_connection(&id) {
                (conn.protocol.clone(), conn.cancellation_token.clone())
            } else {
                return;
            }
        };

        let msg = ServerMessage::ConnectionClosed { reason };
        PROTOCOL_SERVICE
            .get()
            .unwrap()
            .handle_server_message(&protocol, id, &msg)
            .await;

        //wait a moment to allow message to be sent
        tokio::time::sleep(Duration::from_millis(100)).await;
        cancellation_token.cancel();
        log::info!("Client {} closed", id);
    }

    async fn close_all_clients(&self) {
        let client_ids = {
            self.client_registry
                .read()
                .get_connections()
                .keys()
                .cloned()
                .collect::<Vec<_>>()
        };

        let mut futures = vec![];
        for id in client_ids {
            let fut = self.close_with_reason(id, DisconnectReason::ServerShutdown);
            futures.push(fut);
        }
        futures::future::join_all(futures).await;
    }

    pub fn get_protocol(&self, id: ConnectionId) -> Protocol {
        let registry = self.client_registry.read();
        if let Some(conn) = registry.get_connection(&id) {
            conn.protocol.clone()
        } else {
            Protocol::V0
        }
    }

    pub async fn launch_client_cleanup_task(&self, cancellation_token: CancellationToken) {
        let timeout_duration = Duration::from_secs(60 * 30);

        loop {
            select! {
                _ = cancellation_token.cancelled() => {
                    log::info!("Client cleanup task shutting down");
                    break;
                }
                _ = tokio::time::sleep(Duration::from_secs(60)) => {}
            }
            let now = Instant::now();
            let inactive_clients: Vec<ConnectionId> = self
                .client_registry
                .read()
                .get_connections()
                .iter()
                .filter_map(|(id, conn)| {
                    if now.duration_since(conn.last_activity) > timeout_duration {
                        Some(*id)
                    } else {
                        None
                    }
                })
                .collect();
            for client_id in inactive_clients {
                log::info!("Cleaning up inactive client {}", client_id);
                self.close_with_reason(client_id, DisconnectReason::Inactivity)
                    .await;
            }
        }
    }

    pub async fn handle_client_websocket(&self, ws: WebSocket) {
        self.handle_connection(
            ws,
            |s| Message::Binary(s.into()),
            |m| match m {
                Message::Text(t) => Some(ClientMessage::Text(t.to_string())),
                Message::Close(_) => Some(ClientMessage::Close),
                _ => None,
            },
        )
        .await;
    }

    pub async fn handle_client_tcp(&self, tcp: TcpStream) {
        let framed = Framed::new(tcp, LinesCodec::new());
        self.handle_connection(framed, |s| s.to_string(), |s| Some(ClientMessage::Text(s)))
            .await;
    }

    pub async fn run(
        this: Arc<TransportServiceImpl>,
        app: Arc<Application>,
        auth: Arc<dyn AuthenticationPort + Send + Sync + 'static>,
        acl: Arc<LegacyAPIAntiCorruptionLayer>,
        shutdown_signal: impl std::future::Future<Output = ()> + Send + 'static,
    ) {
        let protocol_service = Arc::new(ProtocolService::new(
            app.clone(),
            this.clone(),
            auth.clone(),
            acl,
        ));

        APPLICATION.set(app.clone()).ok().unwrap();
        TRANSPORT_IMPL.set(this.clone()).ok().unwrap();
        PROTOCOL_SERVICE.set(protocol_service.clone()).ok().unwrap();

        let router = Router::new()
            .route("/", any(ws_handler))
            .route("/ws", any(ws_handler));

        let router = crate::protocol::register_http_endpoints(router);

        let ws_port = std::env::var("TAK_WS_PORT")
            .expect("TAK_WS_PORT must be set")
            .parse::<u16>()
            .expect("TAK_WS_PORT must be a valid u16");

        let host = std::env::var("TAK_HOST").expect("TAK_HOST must be set");

        let listener = tokio::net::TcpListener::bind(format!("{}:{}", host, ws_port))
            .await
            .unwrap();

        let cancellation_token = CancellationToken::new();

        let self_clone = this.clone();

        let cancellation_token_clone = cancellation_token.clone();
        let tcp_server_handle = tokio::spawn(async move {
            serve_tcp_server(cancellation_token_clone).await;
        });
        let cancellation_token_clone = cancellation_token.clone();
        let client_cleanup_handle = tokio::spawn(async move {
            this.launch_client_cleanup_task(cancellation_token_clone)
                .await;
        });

        let on_shutdown = async move {
            shutdown_signal.await;
            log::info!("Shutdown signal received, closing all clients");
            self_clone.close_all_clients().await;
            cancellation_token.cancel();
        };

        log::info!("WebSocket server listening on port {}", ws_port);
        axum::serve(listener, router.with_state(app.clone()))
            .with_graceful_shutdown(on_shutdown)
            .await
            .unwrap();

        let (r1, r2) = tokio::join!(tcp_server_handle, client_cleanup_handle);
        if let Err(e1) = r1 {
            log::error!("TCP server task failed: {}", e1);
        }
        if let Err(e2) = r2 {
            log::error!("Client cleanup task failed: {}", e2);
        }

        log::info!("Transport service shut down gracefully");
    }
}

#[derive(Clone, Debug)]
pub enum DisconnectReason {
    ClientQuit,
    Inactivity,
    ServerShutdown,
    Ban(String),
    Kick,
}

#[async_trait::async_trait]
impl ListenerNotificationPort for TransportServiceImpl {
    fn notify_listener(&self, listener: ListenerId, message: ListenerMessage) {
        let registry = self.client_registry.read();
        let Some(client) = registry.get_client(&listener) else {
            return;
        };
        if let Err(e) = client
            .server_message_handler
            .send(ServerMessage::Notification(message.clone()))
        {
            log::error!(
                "Failed to notify listener {}: {}, {:?}",
                listener,
                e.to_string(),
                message
            );
        }
    }

    fn notify_listeners(&self, listeners: &[ListenerId], message: ListenerMessage) {
        for &listener in listeners {
            self.notify_listener(listener, message.clone());
        }
    }

    fn notify_all(&self, message: ListenerMessage) {
        let message = ServerMessage::Notification(message);
        let registry = self.client_registry.read();
        for (listener_id, client) in registry.get_clients() {
            if let Err(e) = client.server_message_handler.send(message.clone()) {
                log::error!(
                    "Failed to notify listener {} of all listeners: {}, {:?}",
                    listener_id,
                    e.to_string(),
                    message
                );
            }
        }
    }
}

#[async_trait::async_trait]
impl PlayerConnectionPort for TransportServiceImpl {
    async fn get_connection_id(&self, player_id: PlayerId) -> Option<ListenerId> {
        let Ok(account_id) = APPLICATION
            .get()
            .unwrap()
            .player_resolver_service
            .resolve_account_id_by_player_id(player_id)
            .await
        else {
            return None;
        };
        self.client_registry
            .read()
            .get_listener(&account_id)
            .copied()
    }
}

static TRANSPORT_IMPL: OnceLock<Arc<TransportServiceImpl>> = OnceLock::new();

async fn ws_handler(ws: WebSocketUpgrade) -> Response {
    ws.protocols(["binary"])
        .on_upgrade(move |socket| async move {
            TRANSPORT_IMPL
                .get()
                .unwrap()
                .handle_client_websocket(socket)
                .await;
        })
}

async fn serve_tcp_server(cancellation_token: CancellationToken) {
    let tcp_port = std::env::var("TAK_TCP_PORT")
        .expect("TAK_TCP_PORT must be set")
        .parse::<u16>()
        .expect("TAK_TCP_PORT must be a valid u16");
    let host = std::env::var("TAK_HOST").expect("TAK_HOST must be set");
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", host, tcp_port))
        .await
        .unwrap();
    log::info!("TCP server listening on port {}", tcp_port);
    let handles = Arc::new(DashMap::new());
    loop {
        let (socket, addr) = select! {
            res = listener.accept() => res.unwrap(),
            _ = cancellation_token.cancelled() => {
                log::info!("TCP server shutting down");
                break;
            }
        };
        log::info!("New TCP connection from {}", addr);
        let conn_id = uuid::Uuid::new_v4();
        let token_clone = cancellation_token.clone();
        let handles_clone = handles.clone();
        let handle = tokio::spawn(async move {
            select! {
                _ = token_clone.cancelled() => {
                    log::info!("Closing TCP connection from {} due to server shutdown", addr);
                    return;
                }
                _ = TRANSPORT_IMPL.get().unwrap().handle_client_tcp(socket)  => {}
            }
            handles_clone.remove(&conn_id);
        });
        handles.insert(conn_id, Some(handle));
    }

    for mut entry in handles.iter_mut() {
        let handle = entry.value_mut();
        if let Some(handle) = handle.take() {
            let _ = handle.await;
        }
    }
}
