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
    ArcProtocolService, ServiceError, ServiceResult,
    player::PlayerUsername,
    protocol::{Protocol, ServerMessage},
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
    let _ = client_service.try_send_to(id, msg.as_ref());
}

#[async_trait::async_trait]
pub trait ClientService {
    fn try_send_to(&self, id: &ClientId, msg: &str) -> Result<(), String>;
    fn associate_player(&self, id: &ClientId, username: &PlayerUsername) -> ServiceResult<()>;
    fn get_associated_player(&self, id: &ClientId) -> Option<PlayerUsername>;
    fn get_associated_client(&self, player: &PlayerUsername) -> Option<ClientId>;
    fn try_protocol_send(&self, id: &ClientId, msg: &ServerMessage);
    fn try_protocol_multicast(&self, ids: &[ClientId], msg: &ServerMessage);
    fn try_auth_protocol_broadcast(&self, msg: &ServerMessage);
    async fn launch_client_cleanup_task(&self);
    async fn handle_client_websocket(&self, ws: WebSocket);
    async fn handle_client_tcp(&self, tcp: TcpStream);
}

#[derive(Clone)]
pub struct ClientServiceImpl {
    protocol_service: ArcProtocolService,
    client_senders: Arc<DashMap<ClientId, (UnboundedSender<String>, CancellationToken)>>,
    client_handlers: Arc<DashMap<ClientId, Protocol>>,
    client_to_player: Arc<DashMap<ClientId, PlayerUsername>>,
    player_to_client: Arc<DashMap<PlayerUsername, ClientId>>,
    last_activity: Arc<DashMap<ClientId, Instant>>,
}

impl ClientServiceImpl {
    pub fn new(protocol_service: ArcProtocolService) -> Self {
        Self {
            protocol_service,
            client_senders: Arc::new(DashMap::new()),
            client_handlers: Arc::new(DashMap::new()),
            client_to_player: Arc::new(DashMap::new()),
            player_to_client: Arc::new(DashMap::new()),
            last_activity: Arc::new(DashMap::new()),
        }
    }

    fn on_disconnect(&self, id: &ClientId) {
        self.client_senders.remove(id);
        self.client_handlers.remove(id);
        self.last_activity.remove(id);
        let player = self.client_to_player.remove(id);
        if let Some((_, username)) = player {
            self.player_to_client.remove(&username);
            self.update_online_players();
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
        let cancellation_token_clone = cancellation_token.clone();

        let client_service = self.clone();
        let receive_task = tokio::spawn(async move {
            client_service
                .handle_receive::<S, M, E>(client_id, ws_receiver, cancellation_token, msg_parser)
                .await;
        });
        let client_service = self.clone();
        let send_task = tokio::spawn(async move {
            client_service
                .handle_send::<S, M>(client_id, ws_sender, cancellation_token_clone, msg_factory)
                .await;
        });
        self.on_connect(&client_id);
        let _ = tokio::join!(receive_task, send_task);
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
        self.client_handlers.insert(id, Protocol::V2);

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
        self.on_disconnect(&id);
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
                    if text.starts_with("Protocol") {
                        self.try_switch_protocol(&id, &text).unwrap_or_else(|e| {
                            println!("Client {} protocol switch error: {}", id, e);
                        });
                    } else if let Some(handler) = self.client_handlers.get(&id) {
                        self.protocol_service
                            .handle_client_message(&handler, &id, text);
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
        let players: Vec<PlayerUsername> = self
            .client_to_player
            .iter()
            .map(|entry| entry.value().clone())
            .collect();
        let msg = ServerMessage::PlayersOnline { players };
        self.try_auth_protocol_broadcast(&msg);
    }

    fn on_connect(&self, id: &ClientId) {
        send_to(self, id, "Welcome!");
        send_to(self, id, "Login or Register");
    }

    fn close_client(&self, id: &ClientId) {
        let Some(cancellation_token) = self.client_senders.get(id).map(|x| x.1.clone()) else {
            return;
        };
        cancellation_token.cancel();
    }
}

#[async_trait::async_trait]
impl ClientService for ClientServiceImpl {
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
        if self.client_to_player.contains_key(id) {
            return ServiceError::not_possible(format!("Player {} already logged in", username));
        }
        if self.player_to_client.contains_key(username) {
            return ServiceError::not_possible(format!(
                "Player {} already logged in from another client",
                username
            ));
        }
        self.client_to_player.insert(*id, username.clone());
        self.player_to_client.insert(username.clone(), *id);
        if let Some(handler) = self.client_handlers.get(id) {
            self.protocol_service
                .on_authenticated(&handler, id, username);
        }
        self.update_online_players();
        Ok(())
    }

    fn get_associated_player(&self, id: &ClientId) -> Option<PlayerUsername> {
        self.client_to_player
            .get(id)
            .map(|entry| entry.value().clone())
    }

    fn get_associated_client(&self, player: &PlayerUsername) -> Option<ClientId> {
        self.player_to_client
            .get(player)
            .map(|entry| *entry.value())
    }

    fn try_protocol_send(&self, id: &ClientId, msg: &ServerMessage) {
        if let Some(handler) = self.client_handlers.get(id) {
            self.protocol_service
                .handle_server_message(&handler, id, msg);
        }
    }

    fn try_protocol_multicast(&self, ids: &[ClientId], msg: &ServerMessage) {
        for id in ids {
            self.try_protocol_send(id, msg);
        }
    }

    fn try_auth_protocol_broadcast(&self, msg: &ServerMessage) {
        for entry in self.client_handlers.iter() {
            if self.client_to_player.contains_key(entry.key()) {
                self.protocol_service
                    .handle_server_message(&entry, entry.key(), msg);
            }
        }
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

    async fn launch_client_cleanup_task(&self) {}
    async fn handle_client_websocket(&self, _ws: WebSocket) {}
    async fn handle_client_tcp(&self, _tcp: TcpStream) {}
}
