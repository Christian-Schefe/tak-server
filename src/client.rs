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
use tak_server_domain::{
    ServiceError, ServiceResult,
    app::LazyAppState,
    game::SpectatorId,
    player::PlayerUsername,
    transport::{DisconnectReason, TransportService},
    util::OneOneDashMap,
};
use tokio::{net::TcpStream, select, sync::mpsc::UnboundedSender};
use tokio_util::{
    codec::{Framed, LinesCodec},
    sync::CancellationToken,
};
use uuid::Uuid;

use crate::protocol::Protocol;
use tak_server_domain::transport::ServerMessage;

pub type ClientId = Uuid;

pub enum ClientMessage {
    Text(String),
    Close,
}

fn new_client() -> ClientId {
    Uuid::new_v4()
}

#[derive(Clone)]
pub struct TransportServiceImpl {
    app_state: LazyAppState,
    client_senders: Arc<DashMap<ClientId, (UnboundedSender<String>, CancellationToken)>>,
    client_handlers: Arc<DashMap<ClientId, Protocol>>,
    player_associations: Arc<OneOneDashMap<ClientId, PlayerUsername>>,
    last_activity: Arc<DashMap<ClientId, Instant>>,
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

    fn on_disconnect(&self, id: &ClientId) {
        self.client_senders.remove(id);
        self.client_handlers.remove(id);
        self.last_activity.remove(id);
        let player = self.player_associations.remove_by_key(id);
        self.app_state
            .player_connection_service()
            .on_spectator_disconnected(id);
        if let Some(username) = player {
            self.app_state
                .player_connection_service()
                .on_player_disconnected(&username);
            println!("Player {} disconnected (client {})", username, id);
        } else {
            println!("Client {} disconnected", id);
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
        let client_id = new_client();
        let cancellation_token = CancellationToken::new();

        let client_service = self.clone();
        let cancellation_token_clone = cancellation_token.clone();
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
        self.on_disconnect(&client_id);
    }

    async fn handle_send<S, M>(
        &self,
        id: ClientId,
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
        self.client_handlers.insert(id, Protocol::V0);

        self.on_connect(&id);

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
        println!("Client {} send ended", id);
        cancellation_token.cancel();
    }

    async fn handle_receive<S, M, E>(
        &self,
        id: ClientId,
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
                    println!("Client {} sent invalid message", id);
                    continue;
                }
            };
            match msg {
                ClientMessage::Text(text) => {
                    if text.to_ascii_lowercase().starts_with("protocol") {
                        self.try_switch_protocol(&id, &text).unwrap_or_else(|e| {
                            println!("Client {} protocol switch error: {}", id, e);
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
                            &id,
                            text,
                        )
                        .await;
                    } else {
                        println!("Client {} has no protocol handler", id);
                    }
                }
                ClientMessage::Close => break,
            }
        }
        println!("Client {} received ended", id);
        cancellation_token.cancel();
    }

    fn try_switch_protocol(&self, id: &ClientId, protocol_msg: &str) -> Result<(), String> {
        let parts: Vec<&str> = protocol_msg.split_whitespace().collect();
        if parts.len() == 2 {
            let protocol = Protocol::from_id(parts[1]).ok_or("Unknown protocol")?;
            if let Some(mut handler) = self.client_handlers.get_mut(id) {
                *handler = protocol.clone();
                drop(handler);
                println!("Client {} set protocol to {:?}", id, parts[1]);

                crate::protocol::on_connected(self.app_state.unwrap(), self, &protocol, &id);
                Ok(())
            } else {
                Err("Client handler not found".into())
            }
        } else {
            Err("Invalid protocol message format".into())
        }
    }

    fn on_connect(&self, id: &ClientId) {
        println!("Client {} connected", id);
        if let Some(handler) = self.client_handlers.get(&id) {
            let protocol = handler.clone();
            drop(handler);
            crate::protocol::on_connected(self.app_state.unwrap(), self, &protocol, &id);
        }
    }

    pub fn try_send_to(&self, id: &ClientId, msg: &str) -> Result<(), String> {
        if let Some(sender) = self.client_senders.get(id) {
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
        id: &ClientId,
        username: &PlayerUsername,
    ) -> ServiceResult<()> {
        if self.player_associations.contains_key(id) {
            return ServiceError::not_possible(format!("Player {} already logged in", username));
        }
        if let Some(prev_client_id) = self.player_associations.get_by_value(username) {
            let _ = self.try_spectator_send(
                &prev_client_id,
                &ServerMessage::ConnectionClosed {
                    reason: DisconnectReason::NewSession,
                },
            );
            self.close_client(&prev_client_id);
            // call on_disconnect directly to clean up immediately
            self.on_disconnect(&prev_client_id);
            println!(
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
        if let Some(handler) = self.client_handlers.get(id) {
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
            .on_player_connected(username);
        Ok(())
    }

    pub fn get_associated_player(&self, id: &ClientId) -> Option<PlayerUsername> {
        self.player_associations.get_by_key(id)
    }

    fn get_associated_client(&self, player: &PlayerUsername) -> Option<ClientId> {
        self.player_associations.get_by_value(player)
    }

    pub fn close_client(&self, id: &ClientId) {
        let Some(entry) = self.client_senders.get(id) else {
            println!("Client {} already closed", id);
            return;
        };
        let token = entry.1.clone();
        drop(entry);
        token.cancel();
        println!("Client {} closed", id);
    }

    pub fn get_protocol(&self, id: &ClientId) -> Protocol {
        self.client_handlers
            .get(id)
            .map(|entry| entry.clone())
            .unwrap_or(Protocol::V0)
    }

    pub async fn launch_client_cleanup_task(&self) {
        let timeout_duration = Duration::from_secs(300);

        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            let now = Instant::now();
            let inactive_clients: Vec<ClientId> = self
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
                println!("Cleaning up inactive client {}", client_id);
                let _ = self.try_spectator_send(
                    &client_id,
                    &ServerMessage::ConnectionClosed {
                        reason: DisconnectReason::NewSession,
                    },
                );
                self.close_client(&client_id);
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

    pub async fn run(self, app: LazyAppState) {
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

        tokio::spawn(async move {
            serve_tcp_server().await;
        });
        tokio::spawn(async move {
            self.launch_client_cleanup_task().await;
        });

        println!("WebSocket server listening on port {}", ws_port);
        axum::serve(listener, router.with_state(app.unwrap().clone()))
            .await
            .unwrap();
    }
}

impl TransportService for TransportServiceImpl {
    fn try_player_send(&self, player: &PlayerUsername, msg: &ServerMessage) {
        if let Some(id) = self.get_associated_client(player) {
            self.try_spectator_send(&id, msg);
        }
    }

    fn try_spectator_send(&self, id: &SpectatorId, msg: &ServerMessage) {
        if let Some(handler) = self.client_handlers.get(id) {
            let protocol = handler.clone();
            drop(handler);
            let app_state = self.app_state.unwrap().clone();
            let transport = self.clone();
            let msg = msg.clone();
            let id = id.clone();
            tokio::spawn(async move {
                crate::protocol::handle_server_message(
                    &app_state, &transport, &protocol, &id, &msg,
                )
                .await;
            });
        }
    }

    fn try_player_broadcast(&self, msg: &ServerMessage) {
        for entry in self.client_handlers.iter() {
            let id = entry.key().clone();
            if self.player_associations.contains_key(&id) {
                let protocol = entry.clone();
                drop(entry);
                let app_state = self.app_state.unwrap().clone();
                let transport = self.clone();
                let msg = msg.clone();
                tokio::spawn(async move {
                    crate::protocol::handle_server_message(
                        &app_state, &transport, &protocol, &id, &msg,
                    )
                    .await;
                });
            }
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

async fn serve_tcp_server() {
    let tcp_port = std::env::var("TAK_TCP_PORT")
        .unwrap_or_else(|_| "10000".to_string())
        .parse::<u16>()
        .expect("TAK_TCP_PORT must be a valid u16");
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", tcp_port))
        .await
        .unwrap();
    println!("TCP server listening on port {}", tcp_port);
    loop {
        let (socket, addr) = listener.accept().await.unwrap();
        println!("New TCP connection from {}", addr);
        tokio::spawn(async move {
            TRANSPORT_IMPL
                .get()
                .unwrap()
                .handle_client_tcp(socket)
                .await;
        });
    }
}
