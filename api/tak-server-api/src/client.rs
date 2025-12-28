use std::{
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
use log::{debug, error, info, warn};

use more_dashmap::one_one::OneOneDashMap;
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

use crate::{
    acl::LegacyAPIAntiCorruptionLayer,
    protocol::{Protocol, ProtocolService},
};

pub enum ClientMessage {
    Text(String),
    Close,
}

#[derive(Clone)]
pub enum ServerMessage {
    Notification(ListenerMessage),
    ConnectionClosed { reason: DisconnectReason },
}

static APPLICATION: OnceLock<Arc<Application>> = OnceLock::new();
static PROTOCOL_SERVICE: OnceLock<Arc<ProtocolService>> = OnceLock::new();

#[derive(Clone)]
pub struct TransportServiceImpl {
    notification_listeners: Arc<DashMap<ListenerId, UnboundedSender<ServerMessage>>>,
    client_senders: Arc<DashMap<ListenerId, (UnboundedSender<String>, CancellationToken)>>,
    client_handlers: Arc<DashMap<ListenerId, Protocol>>,
    account_associations: Arc<OneOneDashMap<ListenerId, AccountId>>,
    last_activity: Arc<DashMap<ListenerId, Instant>>,
}

impl TransportServiceImpl {
    pub fn new() -> Self {
        Self {
            notification_listeners: Arc::new(DashMap::new()),
            client_senders: Arc::new(DashMap::new()),
            client_handlers: Arc::new(DashMap::new()),
            account_associations: Arc::new(OneOneDashMap::new()),
            last_activity: Arc::new(DashMap::new()),
        }
    }

    async fn on_disconnect(&self, id: ListenerId) {
        self.client_senders.remove(&id);
        self.client_handlers.remove(&id);
        self.last_activity.remove(&id);
        let account_id = self.account_associations.remove_by_key(&id);
        if let Some(account_id) = account_id {
            let app = APPLICATION.get().unwrap();
            if let Ok(player_id) = app
                .player_resolver_service
                .resolve_player_id_by_account_id(account_id)
                .await
            {
                app.player_set_online_use_case.set_offline(player_id);
            }
        }
        /*self.application
            .player_connection_service()
            .on_listener_disconnected(id);
        if let Some(username) = player {
            self.application
                .player_connection_service()
                .on_player_disconnected(id, &username)
                .await;
            info!("Player {} disconnected (client {})", username, id);
        } else {
            info!("Client {} disconnected", id);
        }*/
    }

    async fn handle_client<M, S, E>(
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
        let client_id = ListenerId::new();
        let cancellation_token = CancellationToken::new();

        let client_service = self.clone();
        let cancellation_token_clone = cancellation_token.clone();
        self.client_handlers.insert(client_id, Protocol::V0);
        let receive_task = tokio::spawn(async move {
            client_service
                .handle_receive::<S, M, E>(
                    client_id,
                    ws_receiver,
                    cancellation_token_clone,
                    msg_parser,
                )
                .await;
        });

        let client_service = self.clone();
        let cancellation_token_clone = cancellation_token.clone();
        let send_task = tokio::spawn(async move {
            client_service
                .handle_send::<S, M>(client_id, ws_sender, cancellation_token_clone, msg_factory)
                .await;
        });

        let (notification_tx, notification_rx) = tokio::sync::mpsc::unbounded_channel();
        self.notification_listeners
            .insert(client_id, notification_tx);

        let client_service = self.clone();
        let cancellation_token_clone = cancellation_token.clone();
        let notification_task = tokio::spawn(async move {
            client_service
                .handle_notification(notification_rx, cancellation_token_clone, client_id)
                .await;
        });

        let _ = tokio::join!(receive_task, send_task, notification_task);
        self.on_disconnect(client_id).await;
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
            if let Some(handler) = self.client_handlers.get(&id) {
                let protocol = handler.clone();
                drop(handler);

                PROTOCOL_SERVICE
                    .get()
                    .unwrap()
                    .handle_server_message(&protocol, id, &msg)
                    .await;
            }
        }
        info!("Client {} notification ended", id);
    }

    async fn handle_send<S, M>(
        &self,
        id: ListenerId,
        mut ws_sender: impl SinkExt<M> + Unpin + Send + 'static,
        cancellation_token: CancellationToken,
        msg_factory: impl Fn(String) -> M + Send + 'static,
    ) where
        S: futures_util::Sink<M> + Unpin + Send + 'static,
        M: Send + 'static,
    {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        self.client_senders
            .insert(id, (tx, cancellation_token.clone()));

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
        info!("Client {} send ended", id);
        cancellation_token.cancel();
    }

    async fn handle_receive<S, M, E>(
        &self,
        id: ListenerId,
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
            self.last_activity.insert(id, Instant::now());

            let msg = match msg_parser(msg) {
                Some(m) => m,
                None => {
                    info!("Client {} sent invalid message", id);
                    continue;
                }
            };
            match msg {
                ClientMessage::Text(text) => {
                    if text.to_ascii_lowercase().starts_with("protocol") {
                        self.try_switch_protocol(id, &text).unwrap_or_else(|e| {
                            error!("Client {} failed to switch to protocol {}: {}", id, text, e);
                        });
                        // message is still passed to handler to allow protocol to respond.
                    }

                    if let Some(handler) = self.client_handlers.get(&id) {
                        let protocol = handler.clone();
                        drop(handler);
                        PROTOCOL_SERVICE
                            .get()
                            .unwrap()
                            .handle_client_message(&protocol, id, text)
                            .await;
                    } else {
                        error!("Client {} has no protocol handler", id);
                    }
                }
                ClientMessage::Close => break,
            }
        }
        debug!("Client {} received ended", id);
        cancellation_token.cancel();
    }

    fn try_switch_protocol(&self, id: ListenerId, protocol_msg: &str) -> Result<(), String> {
        let parts: Vec<&str> = protocol_msg.split_whitespace().collect();
        if parts.len() == 2 {
            let protocol = Protocol::from_id(parts[1]).ok_or("Unknown protocol")?;
            if let Some(mut handler) = self.client_handlers.get_mut(&id)
                && protocol != *handler
            {
                *handler = protocol.clone();
                drop(handler);
                info!("Client {} set protocol to {:?}", id, parts[1]);

                Ok(())
            } else {
                Err("Client handler not found".into())
            }
        } else {
            Err("Invalid protocol message format".into())
        }
    }

    fn on_connect(&self, id: ListenerId) {
        info!("Client {} connected", id);
        if let Some(handler) = self.client_handlers.get(&id) {
            let protocol = handler.clone();
            drop(handler);
            PROTOCOL_SERVICE.get().unwrap().on_connected(&protocol, id);
        }
    }

    pub fn try_send_to(&self, id: ListenerId, msg: &str) -> Result<(), String> {
        if let Some(sender) = self.client_senders.get(&id) {
            sender
                .0
                .send(msg.into())
                .map_err(|e| format!("Failed to send message: {}", e))
        } else {
            Err("Sender not initialized".into())
        }
    }

    pub async fn associate_account(
        &self,
        id: ListenerId,
        account_id: AccountId,
    ) -> Result<(), String> {
        if self.account_associations.contains_key(&id) {
            return Err(format!("Account {} already logged in", account_id));
        }
        if let Some(prev_client_id) = self.account_associations.get_by_value(&account_id) {
            self.close_with_reason(prev_client_id, DisconnectReason::NewSession)
                .await;
            // call on_disconnect directly to clean up immediately
            self.on_disconnect(prev_client_id).await;
            info!(
                "Disconnected previous session of account {} (client {})",
                account_id, prev_client_id
            );
        }
        if !self.account_associations.try_insert(id, account_id) {
            return Err(format!(
                "Failed to associate account_id {} with client {}",
                account_id, id
            ));
        }
        if let Some(handler) = self.client_handlers.get(&id) {
            PROTOCOL_SERVICE
                .get()
                .unwrap()
                .on_authenticated(&handler, id, account_id)
                .await;
        }
        let app = APPLICATION.get().unwrap();
        if let Ok(player_id) = app
            .player_resolver_service
            .resolve_player_id_by_account_id(account_id)
            .await
        {
            app.player_set_online_use_case.set_online(player_id);
        }
        Ok(())
    }

    pub fn get_associated_account(&self, id: ListenerId) -> Option<AccountId> {
        self.account_associations.get_by_key(&id)
    }

    pub async fn get_associated_player_and_account(
        &self,
        id: ListenerId,
    ) -> Option<(PlayerId, AccountId)> {
        let app = APPLICATION.get().unwrap();
        let account_id = self.account_associations.get_by_key(&id)?;
        let player_id = app
            .player_resolver_service
            .resolve_player_id_by_account_id(account_id)
            .await
            .ok()?;
        Some((player_id, account_id))
    }

    pub async fn close_with_reason(&self, id: ListenerId, reason: DisconnectReason) {
        if let Some(sender) = self.notification_listeners.get(&id) {
            if let Err(e) = sender.send(ServerMessage::ConnectionClosed { reason }) {
                error!("Failed to notify listener {}: {}", id, e.to_string());
            }
        }
        //self.try_listener_send(id, &ServerMessage::ConnectionClosed { reason })
        //    .await;
        //wait a moment to allow message to be sent
        tokio::time::sleep(Duration::from_millis(100)).await;
        self.close_client(id);
    }

    fn close_client(&self, id: ListenerId) {
        let Some(entry) = self.client_senders.get(&id) else {
            warn!("Client {} already closed", id);
            return;
        };
        let token = entry.1.clone();
        drop(entry);
        token.cancel();
        info!("Client {} closed", id);
    }

    async fn close_all_clients(&self) {
        let client_ids: Vec<ListenerId> = self
            .client_senders
            .iter()
            .map(|entry| *entry.key())
            .collect();

        let mut futures = vec![];
        for id in client_ids {
            let fut = self.close_with_reason(id, DisconnectReason::ServerShutdown);
            futures.push(fut);
        }
        futures::future::join_all(futures).await;
    }

    pub fn get_protocol(&self, id: ListenerId) -> Protocol {
        self.client_handlers
            .get(&id)
            .map(|entry| entry.clone())
            .unwrap_or(Protocol::V0)
    }

    pub async fn launch_client_cleanup_task(&self, cancellation_token: CancellationToken) {
        let timeout_duration = Duration::from_secs(300);

        loop {
            select! {
                _ = cancellation_token.cancelled() => {
                    info!("Client cleanup task shutting down");
                    break;
                }
                _ = tokio::time::sleep(Duration::from_secs(60)) => {}
            }
            let now = Instant::now();
            let inactive_clients: Vec<ListenerId> = self
                .last_activity
                .iter()
                .filter_map(|entry| {
                    if now.duration_since(*entry.value()) > timeout_duration {
                        Some(*entry.key())
                    } else {
                        None
                    }
                })
                .collect();
            for client_id in inactive_clients {
                info!("Cleaning up inactive client {}", client_id);
                self.close_with_reason(client_id, DisconnectReason::Inactivity)
                    .await;
            }
        }
    }

    pub async fn handle_client_websocket(&self, ws: WebSocket) {
        self.handle_client(
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
        self.handle_client(framed, |s| s.to_string(), |s| Some(ClientMessage::Text(s)))
            .await;
    }

    pub async fn run(
        self,
        app: Arc<Application>,
        auth: Arc<dyn AuthenticationPort + Send + Sync + 'static>,
        acl: Arc<LegacyAPIAntiCorruptionLayer>,
        shutdown_signal: impl std::future::Future<Output = ()> + Send + 'static,
    ) {
        let transport_impl = Arc::new(self.clone());
        let protocol_service = Arc::new(ProtocolService::new(
            app.clone(),
            transport_impl.clone(),
            auth.clone(),
            acl,
        ));

        APPLICATION.set(app.clone()).ok().unwrap();
        TRANSPORT_IMPL.set(transport_impl.clone()).ok().unwrap();
        PROTOCOL_SERVICE.set(protocol_service.clone()).ok().unwrap();

        let router = Router::new()
            .route("/", any(ws_handler))
            .route("/ws", any(ws_handler));

        let router = crate::protocol::register_http_endpoints(router);

        let ws_port = std::env::var("TAK_WS_PORT")
            .unwrap_or_else(|_| "9999".to_string())
            .parse::<u16>()
            .expect("TAK_WS_PORT must be a valid u16");

        let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", ws_port))
            .await
            .unwrap();

        let cancellation_token = CancellationToken::new();

        let self_clone = self.clone();

        let cancellation_token_clone = cancellation_token.clone();
        let tcp_server_handle = tokio::spawn(async move {
            serve_tcp_server(cancellation_token_clone).await;
        });
        let cancellation_token_clone = cancellation_token.clone();
        let client_cleanup_handle = tokio::spawn(async move {
            self.launch_client_cleanup_task(cancellation_token_clone)
                .await;
        });

        let on_shutdown = async move {
            shutdown_signal.await;
            info!("Shutdown signal received, closing all clients");
            self_clone.close_all_clients().await;
            cancellation_token.cancel();
        };

        info!("WebSocket server listening on port {}", ws_port);
        axum::serve(listener, router.with_state(app.clone()))
            .with_graceful_shutdown(on_shutdown)
            .await
            .unwrap();

        let (r1, r2) = tokio::join!(tcp_server_handle, client_cleanup_handle);
        if let Err(e1) = r1 {
            error!("TCP server task failed: {}", e1);
        }
        if let Err(e2) = r2 {
            error!("Client cleanup task failed: {}", e2);
        }

        info!("Transport service shut down gracefully");
    }
}

#[derive(Clone, Debug)]
pub enum DisconnectReason {
    ClientQuit,
    Inactivity,
    NewSession,
    ServerShutdown,
    Ban(String),
    Kick,
}

#[async_trait::async_trait]
impl ListenerNotificationPort for TransportServiceImpl {
    fn notify_listener(&self, listener: ListenerId, message: ListenerMessage) {
        if let Some(sender) = self.notification_listeners.get(&listener) {
            if let Err(e) = sender.send(ServerMessage::Notification(message)) {
                error!("Failed to notify listener {}: {}", listener, e.to_string());
            }
        }
    }

    fn notify_listeners(&self, listeners: &[ListenerId], message: ListenerMessage) {
        for &listener in listeners {
            self.notify_listener(listener, message.clone());
        }
    }

    fn notify_all(&self, message: ListenerMessage) {
        let message = ServerMessage::Notification(message);
        for entry in self.notification_listeners.iter() {
            let listener_id = *entry.key();
            let sender = entry.value();
            if let Err(e) = sender.send(message.clone()) {
                error!(
                    "Failed to notify listener {}: {}",
                    listener_id,
                    e.to_string()
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
        self.account_associations.get_by_value(&account_id)
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
        .unwrap_or_else(|_| "10000".to_string())
        .parse::<u16>()
        .expect("TAK_TCP_PORT must be a valid u16");
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", tcp_port))
        .await
        .unwrap();
    info!("TCP server listening on port {}", tcp_port);
    let handles = Arc::new(DashMap::new());
    loop {
        let (socket, addr) = select! {
            res = listener.accept() => res.unwrap(),
            _ = cancellation_token.cancelled() => {
                info!("TCP server shutting down");
                break;
            }
        };
        info!("New TCP connection from {}", addr);
        let conn_id = uuid::Uuid::new_v4();
        let token_clone = cancellation_token.clone();
        let handles_clone = handles.clone();
        let handle = tokio::spawn(async move {
            select! {
                _ = token_clone.cancelled() => {
                    info!("Closing TCP connection from {} due to server shutdown", addr);
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
