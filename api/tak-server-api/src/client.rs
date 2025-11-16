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
    routing::{any, post},
};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use tak_server_domain::{
    ServiceError, ServiceResult,
    app::LazyAppState,
    player::PlayerUsername,
    transport::{DisconnectReason, ListenerId, TransportService},
    util::OneOneDashMap,
};
use tokio::{net::TcpStream, select, sync::mpsc::UnboundedSender};
use tokio_util::{
    codec::{Framed, LinesCodec},
    sync::CancellationToken,
};

use crate::protocol::Protocol;
use tak_server_domain::transport::ServerMessage;

pub enum ClientMessage {
    Text(String),
    Close,
}

#[derive(Clone)]
pub struct TransportServiceImpl {
    app_state: LazyAppState,
    client_senders: Arc<DashMap<ListenerId, (UnboundedSender<String>, CancellationToken)>>,
    client_handlers: Arc<DashMap<ListenerId, Protocol>>,
    player_associations: Arc<OneOneDashMap<ListenerId, PlayerUsername>>,
    last_activity: Arc<DashMap<ListenerId, Instant>>,
}

impl TransportServiceImpl {
    pub fn new(app_state: LazyAppState) -> Self {
        Self {
            app_state,
            client_senders: Arc::new(DashMap::new()),
            client_handlers: Arc::new(DashMap::new()),
            player_associations: Arc::new(OneOneDashMap::new()),
            last_activity: Arc::new(DashMap::new()),
        }
    }

    async fn on_disconnect(&self, id: ListenerId) {
        self.client_senders.remove(&id);
        self.client_handlers.remove(&id);
        self.last_activity.remove(&id);
        let player = self.player_associations.remove_by_key(&id);
        self.app_state
            .player_connection_service()
            .on_listener_disconnected(id);
        if let Some(username) = player {
            self.app_state
                .player_connection_service()
                .on_player_disconnected(id, &username)
                .await;
            info!("Player {} disconnected (client {})", username, id);
        } else {
            info!("Client {} disconnected", id);
        }
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

        let _ = tokio::join!(receive_task, send_task);
        self.on_disconnect(client_id).await;
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
                        crate::protocol::handle_client_message(
                            self.app_state.unwrap(),
                            self,
                            &protocol,
                            id,
                            text,
                        )
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
            crate::protocol::on_connected(self.app_state.unwrap(), self, &protocol, id);
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

    pub async fn associate_player(
        &self,
        id: ListenerId,
        username: &PlayerUsername,
    ) -> ServiceResult<()> {
        if self.player_associations.contains_key(&id) {
            return ServiceError::not_possible(format!("Player {} already logged in", username));
        }
        if let Some(prev_client_id) = self.player_associations.get_by_value(username) {
            self.close_with_reason(prev_client_id, DisconnectReason::NewSession)
                .await;
            // call on_disconnect directly to clean up immediately
            self.on_disconnect(prev_client_id).await;
            info!(
                "Disconnected previous session of player {} (client {})",
                username, prev_client_id
            );
        }
        if !self
            .player_associations
            .try_insert(id.clone(), username.clone())
        {
            return ServiceError::internal(format!(
                "Failed to associate player {} with client {}",
                username, id
            ));
        }
        if let Some(handler) = self.client_handlers.get(&id) {
            crate::protocol::on_authenticated(
                self.app_state.unwrap(),
                self,
                &handler,
                id,
                username,
            )
            .await;
        }
        self.app_state
            .player_connection_service()
            .on_player_connected(id, username)
            .await;
        Ok(())
    }

    pub fn get_associated_player(&self, id: ListenerId) -> Option<PlayerUsername> {
        self.player_associations.get_by_key(&id)
    }

    pub async fn close_with_reason(&self, id: ListenerId, reason: DisconnectReason) {
        self.try_listener_send(id, &ServerMessage::ConnectionClosed { reason })
            .await;
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
        app: LazyAppState,
        shutdown_signal: impl std::future::Future<Output = ()> + Send + 'static,
    ) {
        TRANSPORT_IMPL.set(Arc::new(self.clone())).ok().unwrap();

        let router = Router::new()
            .route("/", any(ws_handler))
            .route("/ws", any(ws_handler))
            .route("/auth/login", post(crate::jwt::handle_login));

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
        axum::serve(listener, router.with_state(app.unwrap().clone()))
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

#[async_trait::async_trait]
impl TransportService for TransportServiceImpl {
    async fn disconnect_listener(&self, id: ListenerId, reason: DisconnectReason) {
        self.close_with_reason(id, reason).await;
    }
    async fn try_listener_send(&self, id: ListenerId, msg: &ServerMessage) {
        if let Some(handler) = self.client_handlers.get(&id) {
            let protocol = handler.clone();
            drop(handler);

            crate::protocol::handle_server_message(
                self.app_state.unwrap(),
                self,
                &protocol,
                id,
                msg,
            )
            .await;
        }
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
