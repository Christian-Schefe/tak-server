use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::extract::ws::{Message, WebSocket};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use tokio::{net::TcpStream, select, sync::mpsc::UnboundedSender};
use tokio_util::{
    codec::{Framed, LinesCodec},
    sync::CancellationToken,
};
use uuid::Uuid;

use crate::{
    AppState, ArcChatService, ArcGameService, ArcProtocolService, ArcSeekService, ServiceError,
    ServiceResult,
    player::PlayerUsername,
    protocol::{DisconnectReason, Protocol, ServerMessage},
    util::{LazyInit, OneOneDashMap},
};

pub type ClientId = Uuid;

pub enum ClientMessage {
    Text(String),
    Close,
}

fn new_client() -> ClientId {
    Uuid::new_v4()
}

pub fn send_to<T>(client_service: &dyn ClientService, id: &ClientId, msg: T)
where
    T: AsRef<str>,
{
    if let Err(e) = client_service.try_send_to(id, msg.as_ref()) {
        println!("Failed to send message to client {}: {}", id, e);
    }
}

#[async_trait::async_trait]
pub trait ClientService {
    fn init(&self, app: &AppState);
    fn try_send_to(&self, id: &ClientId, msg: &str) -> Result<(), String>;
    fn associate_player(&self, id: &ClientId, username: &PlayerUsername) -> ServiceResult<()>;
    fn get_associated_player(&self, id: &ClientId) -> Option<PlayerUsername>;
    fn get_associated_client(&self, player: &PlayerUsername) -> Option<ClientId>;
    fn try_protocol_send(&self, id: &ClientId, msg: &ServerMessage);
    fn try_protocol_multicast(&self, ids: &[ClientId], msg: &ServerMessage);
    fn try_auth_protocol_broadcast(&self, msg: &ServerMessage);
    fn close_client(&self, id: &ClientId);
    fn get_offline_since(&self, player: &PlayerUsername) -> Result<Option<Instant>, ()>;
    fn get_protocol(&self, id: &ClientId) -> Protocol;
    async fn launch_client_cleanup_task(&self);
    async fn handle_client_websocket(&self, ws: WebSocket);
    async fn handle_client_tcp(&self, tcp: TcpStream);
}

#[derive(Clone)]
pub struct ClientServiceImpl {
    protocol_service: ArcProtocolService,
    seek_service: LazyInit<ArcSeekService>,
    game_service: LazyInit<ArcGameService>,
    chat_service: LazyInit<ArcChatService>,
    client_senders: Arc<DashMap<ClientId, (UnboundedSender<String>, CancellationToken)>>,
    client_handlers: Arc<DashMap<ClientId, Protocol>>,
    player_associations: Arc<OneOneDashMap<ClientId, PlayerUsername>>,
    last_activity: Arc<DashMap<ClientId, Instant>>,
    last_disconnect: Arc<moka::sync::Cache<PlayerUsername, Instant>>,
}

impl ClientServiceImpl {
    pub fn new(protocol_service: ArcProtocolService) -> Self {
        Self {
            protocol_service,
            seek_service: LazyInit::new(),
            game_service: LazyInit::new(),
            chat_service: LazyInit::new(),
            client_senders: Arc::new(DashMap::new()),
            client_handlers: Arc::new(DashMap::new()),
            player_associations: Arc::new(OneOneDashMap::new()),
            last_activity: Arc::new(DashMap::new()),
            last_disconnect: Arc::new(
                moka::sync::Cache::builder()
                    .time_to_live(Duration::from_secs(600))
                    .build(),
            ),
        }
    }

    fn on_disconnect(&self, id: &ClientId) {
        self.client_senders.remove(id);
        self.client_handlers.remove(id);
        self.last_activity.remove(id);
        let player = self.player_associations.remove_by_key(id);
        let _ = self.chat_service.get().leave_all_rooms(id);
        let _ = self.game_service.get().unobserve_all(&id);
        if let Some(username) = player {
            let _ = self.seek_service.get().remove_seek_of_player(&username);
            self.update_online_players();
            self.last_disconnect
                .insert(username.clone(), Instant::now());
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
        E: 'static,
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
                        self.protocol_service
                            .handle_client_message(&protocol, &id, text);
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
                *handler = protocol;
                println!("Client {} set protocol to {:?}", id, parts[1]);
                Ok(())
            } else {
                Err("Client handler not found".into())
            }
        } else {
            Err("Invalid protocol message format".into())
        }
    }

    fn update_online_players(&self) {
        let players = self.player_associations.get_values();
        let msg = ServerMessage::PlayersOnline { players };
        self.try_auth_protocol_broadcast(&msg);
    }

    fn on_connect(&self, id: &ClientId) {
        println!("Client {} connected", id);
        send_to(self, id, "Welcome!");
        send_to(self, id, "Login or Register");
    }
}

#[async_trait::async_trait]
impl ClientService for ClientServiceImpl {
    fn init(&self, app: &AppState) {
        let _ = self.seek_service.init(app.seek_service.clone());
        let _ = self.chat_service.init(app.chat_service.clone());
        let _ = self.game_service.init(app.game_service.clone());
    }

    fn try_send_to(&self, id: &ClientId, msg: &str) -> Result<(), String> {
        if let Some(sender) = self.client_senders.get(id) {
            sender
                .0
                .send(msg.into())
                .map_err(|e| format!("Failed to send message: {}", e))
        } else {
            Err("Sender not initialized".into())
        }
    }

    fn associate_player(&self, id: &ClientId, username: &PlayerUsername) -> ServiceResult<()> {
        if self.player_associations.contains_key(id) {
            return ServiceError::not_possible(format!("Player {} already logged in", username));
        }
        if let Some(prev_client_id) = self.player_associations.get_by_value(username) {
            let _ = self.try_protocol_send(
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
            self.protocol_service
                .on_authenticated(&handler, id, username);
        }
        self.update_online_players();
        Ok(())
    }

    fn get_associated_player(&self, id: &ClientId) -> Option<PlayerUsername> {
        self.player_associations.get_by_key(id)
    }

    fn get_associated_client(&self, player: &PlayerUsername) -> Option<ClientId> {
        self.player_associations.get_by_value(player)
    }

    fn try_protocol_send(&self, id: &ClientId, msg: &ServerMessage) {
        if let Some(handler) = self.client_handlers.get(id) {
            let protocol = handler.clone();
            drop(handler);
            self.protocol_service
                .handle_server_message(&protocol, id, msg);
        }
    }

    fn try_protocol_multicast(&self, ids: &[ClientId], msg: &ServerMessage) {
        for id in ids {
            self.try_protocol_send(id, msg);
        }
    }

    fn try_auth_protocol_broadcast(&self, msg: &ServerMessage) {
        for entry in self.client_handlers.iter() {
            let id = entry.key().clone();
            if self.player_associations.contains_key(&id) {
                let protocol = entry.clone();
                drop(entry);
                self.protocol_service
                    .handle_server_message(&protocol, &id, msg);
            }
        }
    }

    fn close_client(&self, id: &ClientId) {
        let Some(entry) = self.client_senders.get(id) else {
            println!("Client {} already closed", id);
            return;
        };
        let token = entry.1.clone();
        drop(entry);
        token.cancel();
        println!("Client {} closed", id);
    }

    fn get_offline_since(&self, player: &PlayerUsername) -> Result<Option<Instant>, ()> {
        if self.player_associations.contains_value(player) {
            Err(())
        } else {
            Ok(self.last_disconnect.get(player))
        }
    }

    fn get_protocol(&self, id: &ClientId) -> Protocol {
        self.client_handlers
            .get(id)
            .map(|entry| entry.clone())
            .unwrap_or(Protocol::V0)
    }

    async fn launch_client_cleanup_task(&self) {
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
                let _ = self.try_protocol_send(
                    &client_id,
                    &ServerMessage::ConnectionClosed {
                        reason: DisconnectReason::NewSession,
                    },
                );
                self.close_client(&client_id);
            }
        }
    }

    async fn handle_client_websocket(&self, ws: WebSocket) {
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

    async fn handle_client_tcp(&self, tcp: TcpStream) {
        let framed = Framed::new(tcp, LinesCodec::new());
        self.handle_client(framed, |s| s.to_string(), |s| Some(ClientMessage::Text(s)))
            .await;
    }
}

#[derive(Clone, Default)]
pub struct MockClientService {
    pub sent_messages: Arc<Mutex<Vec<(ClientId, ServerMessage)>>>,
    pub sent_broadcasts: Arc<Mutex<Vec<ServerMessage>>>,
    pub associated_players: Arc<Mutex<Vec<(ClientId, PlayerUsername)>>>,
}

#[allow(unused)]
impl MockClientService {
    pub fn get_messages(&self) -> Vec<(ClientId, ServerMessage)> {
        self.sent_messages.lock().unwrap().clone()
    }

    pub fn get_broadcasts(&self) -> Vec<ServerMessage> {
        self.sent_broadcasts.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl ClientService for MockClientService {
    fn init(&self, _app: &AppState) {}
    fn try_send_to(&self, _id: &ClientId, _msg: &str) -> Result<(), String> {
        Ok(())
    }

    fn associate_player(&self, id: &ClientId, username: &PlayerUsername) -> ServiceResult<()> {
        let mut assoc = self.associated_players.lock().unwrap();
        assoc.push((id.clone(), username.clone()));
        Ok(())
    }

    fn get_associated_player(&self, id: &ClientId) -> Option<PlayerUsername> {
        self.associated_players
            .lock()
            .unwrap()
            .iter()
            .find(|(cid, _)| cid == id)
            .map(|(_, username)| username.clone())
    }

    fn get_associated_client(&self, player: &PlayerUsername) -> Option<ClientId> {
        self.associated_players
            .lock()
            .unwrap()
            .iter()
            .find(|(_, username)| username == player)
            .map(|(cid, _)| cid.clone())
    }

    fn try_protocol_send(&self, id: &ClientId, msg: &ServerMessage) {
        let mut sent = self.sent_messages.lock().unwrap();
        sent.push((id.clone(), msg.clone()));
    }

    fn try_protocol_multicast(&self, ids: &[ClientId], msg: &ServerMessage) {
        let mut sent = self.sent_messages.lock().unwrap();
        for id in ids {
            sent.push((id.clone(), msg.clone()));
        }
    }

    fn try_auth_protocol_broadcast(&self, msg: &ServerMessage) {
        let mut sent = self.sent_broadcasts.lock().unwrap();
        sent.push(msg.clone());
    }

    fn close_client(&self, _id: &ClientId) {}

    fn get_offline_since(&self, _player: &PlayerUsername) -> Result<Option<Instant>, ()> {
        Ok(None)
    }

    fn get_protocol(&self, _id: &ClientId) -> Protocol {
        Protocol::V0
    }

    async fn launch_client_cleanup_task(&self) {}
    async fn handle_client_websocket(&self, _ws: WebSocket) {}
    async fn handle_client_tcp(&self, _tcp: TcpStream) {}
}
