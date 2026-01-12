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

use tak_player_connection::{ConnectionId, PlayerConnectionDriver, PlayerSimpleConnectionPort};
use tak_server_app::{
    Application,
    domain::{AccountId, PlayerId},
    ports::{authentication::AuthenticationPort, notification::ListenerMessage},
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

#[derive(Clone, Debug)]
pub enum ServerMessage {
    Notification(ListenerMessage),
    ConnectionClosed { reason: DisconnectReason },
}

static APPLICATION: OnceLock<Arc<AppState>> = OnceLock::new();

struct AppState {
    app: Arc<Application>,
    protocol_service: Arc<ProtocolService>,
    connection_driver: Arc<PlayerConnectionDriver>,
}

pub struct ClientConnection {
    id: ConnectionId,
    sender: UnboundedSender<String>,
    notification_sender: UnboundedSender<ListenerMessage>,
    cancellation_token: CancellationToken,
    last_activity: Instant,
    protocol: Protocol,
}

#[derive(Clone)]
pub struct TransportServiceImpl {
    connections: Arc<DashMap<ConnectionId, ClientConnection>>,
}

impl TransportServiceImpl {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
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

        let (notify_tx, notify_rx) = tokio::sync::mpsc::unbounded_channel::<ListenerMessage>();
        let client_service = self.clone();
        let cancellation_token_clone = cancellation_token.clone();
        let notification_task = tokio::spawn(async move {
            client_service
                .handle_notification(connection_id, notify_rx, cancellation_token_clone)
                .await;
        });

        let connection = ClientConnection {
            id: connection_id,
            sender: send_tx,
            notification_sender: notify_tx,
            cancellation_token: cancellation_token.clone(),
            last_activity: Instant::now(),
            protocol: Protocol::V0,
        };

        self.connections.insert(connection_id, connection);

        let _ = tokio::join!(receive_task, send_task, notification_task);

        self.connections.remove(&connection_id);

        log::info!("Client {} fully disconnected", connection_id);
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

    async fn handle_notification(
        &self,
        id: ConnectionId,
        mut rx: UnboundedReceiver<ListenerMessage>,
        cancellation_token: CancellationToken,
    ) {
        while let Some(msg) = select! {
            msg = rx.recv() => msg,
            _ = cancellation_token.cancelled() => None,
        } {
            if let Some(conn) = self.connections.get(&id) {
                let protocol = conn.protocol;
                drop(conn);
                APPLICATION
                    .get()
                    .unwrap()
                    .protocol_service
                    .handle_server_message(&protocol, id, &ServerMessage::Notification(msg))
                    .await;
            } else {
                log::error!("Client {} connection not found for notification", id);
                break;
            }
        }
        log::info!("Client {} notification handler ended", id);
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
            //TODO: update last activity time

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

                    let Some(protocol) = self.connections.get(&id).map(|c| c.protocol.clone())
                    else {
                        log::error!("Client {} protocol not found", id);
                        continue;
                    };

                    APPLICATION
                        .get()
                        .unwrap()
                        .protocol_service
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
            if let Some(mut conn) = self.connections.get_mut(&id) {
                conn.protocol = protocol;
                log::info!("Client {} switched to protocol {:?}", id, protocol);
            } else {
                return Err("Connection not found".into());
            }
            Ok(())
        } else {
            Err("Invalid protocol message format".into())
        }
    }

    fn on_connect(&self, id: ConnectionId) {
        log::info!("Client {} connected", id);
        if let Some(conn) = self.connections.get(&id) {
            let protocol = conn.protocol.clone();
            drop(conn);
            APPLICATION
                .get()
                .unwrap()
                .protocol_service
                .on_connected(&protocol, id);
        }
    }

    pub fn try_send_to(&self, id: ConnectionId, msg: &str) -> Result<(), String> {
        if let Some(conn) = self.connections.get(&id) {
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
        if !APPLICATION
            .get()
            .unwrap()
            .connection_driver
            .add_connection(&account_id, id)
            .await
        {
            return Err("Already logged in".into());
        }

        if let Some(protocol) = { self.connections.get(&id).map(|c| c.protocol.clone()) } {
            APPLICATION
                .get()
                .unwrap()
                .protocol_service
                .on_authenticated(&protocol, id, &account_id)
                .await;
        }

        Ok(())
    }

    pub async fn get_associated_player_and_account(
        &self,
        id: ConnectionId,
    ) -> Option<(PlayerId, AccountId)> {
        let app = APPLICATION.get().unwrap();
        let account_id = app.connection_driver.get_account_id(&id)?;
        let player_id = app
            .app
            .player_resolver_service
            .resolve_player_id_by_account_id(&account_id)
            .await
            .ok()?;
        Some((player_id, account_id))
    }

    pub async fn close_with_reason(&self, id: ConnectionId, reason: DisconnectReason) {
        let Some(conn) = self.connections.get(&id) else {
            log::info!("Client {} already disconnected", id);
            return;
        };
        let protocol = conn.protocol.clone();
        let cancellation_token = conn.cancellation_token.clone();
        drop(conn);

        let msg = ServerMessage::ConnectionClosed { reason };
        APPLICATION
            .get()
            .unwrap()
            .protocol_service
            .handle_server_message(&protocol, id, &msg)
            .await;

        //wait a moment to allow message to be sent
        tokio::time::sleep(Duration::from_millis(100)).await;
        cancellation_token.cancel();
        log::info!("Client {} closed", id);
    }

    async fn close_all_clients(&self) {
        let connections = {
            self.connections
                .iter()
                .map(|entry| *entry.key())
                .collect::<Vec<_>>()
        };

        let mut futures = vec![];
        for id in connections {
            let fut = self.close_with_reason(id, DisconnectReason::ServerShutdown);
            futures.push(fut);
        }
        futures::future::join_all(futures).await;
    }

    pub fn get_protocol(&self, id: ConnectionId) -> Protocol {
        if let Some(conn) = self.connections.get(&id) {
            conn.protocol
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
            let inactive_clients: Vec<ConnectionId> = {
                self.connections
                    .iter()
                    .filter_map(|entry| {
                        let id = *entry.key();
                        let conn = entry.value();
                        if now.duration_since(conn.last_activity) > timeout_duration {
                            Some(id)
                        } else {
                            None
                        }
                    })
                    .collect()
            };
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
        connection_driver: Arc<PlayerConnectionDriver>,
        shutdown_signal: impl std::future::Future<Output = ()> + Send + 'static,
    ) {
        let protocol_service = Arc::new(ProtocolService::new(
            app.clone(),
            this.clone(),
            auth.clone(),
            acl,
        ));

        let app_state = Arc::new(AppState {
            app: app.clone(),
            protocol_service,
            connection_driver,
        });

        APPLICATION.set(app_state.clone()).ok().unwrap();
        TRANSPORT_IMPL.set(this.clone()).ok().unwrap();

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

impl PlayerSimpleConnectionPort for TransportServiceImpl {
    fn notify_connection(&self, connection_id: ConnectionId, message: &ListenerMessage) {
        if let Some(conn) = self.connections.get(&connection_id) {
            let _ = conn.notification_sender.send(message.clone());
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
