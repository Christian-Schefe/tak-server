use std::sync::{Arc, LazyLock};

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
    player::PlayerUsername,
    protocol::{BoxedProtocolHandler, ProtocolHandler, ProtocolJSONHandler, ProtocolV2Handler},
    seek::send_seeks_to,
};

static CLIENT_SENDERS: LazyLock<Arc<DashMap<ClientId, UnboundedSender<String>>>> =
    LazyLock::new(|| Arc::new(DashMap::new()));

static CLIENT_HANDLERS: LazyLock<Arc<DashMap<ClientId, BoxedProtocolHandler>>> =
    LazyLock::new(|| Arc::new(DashMap::new()));

static CLIENT_TO_PLAYER: LazyLock<Arc<DashMap<ClientId, PlayerUsername>>> =
    LazyLock::new(|| Arc::new(DashMap::new()));

static PLAYER_TO_CLIENT: LazyLock<Arc<DashMap<PlayerUsername, ClientId>>> =
    LazyLock::new(|| Arc::new(DashMap::new()));

pub type ClientId = Uuid;

pub enum Protocol {
    V2,
    JSON,
}

impl Protocol {
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "2" => Some(Protocol::V2),
            "3" => Some(Protocol::JSON),
            _ => None,
        }
    }
    pub fn get_handler(&self, id: ClientId) -> BoxedProtocolHandler {
        match self {
            Protocol::V2 => Box::new(ProtocolV2Handler::new(id)),
            Protocol::JSON => Box::new(ProtocolJSONHandler::new(id)),
        }
    }
}

pub enum ClientMessage {
    Text(String),
    Close,
}

pub fn new_client() -> ClientId {
    Uuid::new_v4()
}

fn deregister(id: &ClientId) {
    CLIENT_SENDERS.remove(id);
    CLIENT_HANDLERS.remove(id);
    let player = CLIENT_TO_PLAYER.remove(id);
    if let Some((_, username)) = player {
        PLAYER_TO_CLIENT.remove(&username);
    }
}

pub fn handle_client_websocket(ws: WebSocket) {
    handle_client(
        ws,
        |s| Message::Text(s.into()),
        |m| match m {
            Message::Text(t) => Some(ClientMessage::Text(t.to_string())),
            Message::Close(_) => Some(ClientMessage::Close),
            _ => None,
        },
    );
}

pub fn handle_client_tcp(tcp: TcpStream) {
    let framed = Framed::new(tcp, LinesCodec::new());
    handle_client(framed, |s| s.to_string(), |s| Some(ClientMessage::Text(s)));
}

fn handle_client<M, S, E>(
    socket: S,
    msg_factory: impl Fn(String) -> M + Send + 'static,
    msg_parser: impl Fn(M) -> Option<ClientMessage> + Send + 'static,
) where
    S: futures_util::Sink<M> + futures_util::Stream<Item = Result<M, E>> + Unpin + Send + 'static,
    M: Send + 'static,
{
    let (ws_sender, ws_receiver) = socket.split();
    let client_id = new_client();
    let cancellation_token = CancellationToken::new();
    let cancellation_token_clone = cancellation_token.clone();
    handle_receive::<S, M, E>(client_id, ws_receiver, cancellation_token, msg_parser);
    handle_send::<S, M>(client_id, ws_sender, cancellation_token_clone, msg_factory);
    on_connect(&client_id);
}

pub fn handle_send<S, M>(
    id: ClientId,
    mut ws_sender: impl SinkExt<M> + Unpin + Send + 'static,
    cancellation_token: CancellationToken,
    msg_factory: impl Fn(String) -> M + Send + 'static,
) where
    S: futures_util::Sink<M> + Unpin + Send + 'static,
    M: Send + 'static,
{
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    CLIENT_SENDERS.insert(id, tx);
    CLIENT_HANDLERS.insert(id, Protocol::V2.get_handler(id));

    tokio::spawn(async move {
        while let Some(msg) = select! {
            msg = rx.recv() => msg,
            _ = cancellation_token.cancelled() => None,
        } {
            let msg = msg_factory(msg);
            if ws_sender.send(msg).await.is_err() {
                break;
            }
        }
        deregister(&id);
        let _ = ws_sender.close().await;
        println!("Client {} send ended", id);
        cancellation_token.cancel();
    });
}

pub fn handle_receive<S, M, E>(
    id: ClientId,
    mut ws_receiver: impl StreamExt<Item = Result<M, E>> + Unpin + Send + 'static,
    cancellation_token: CancellationToken,
    msg_parser: impl Fn(M) -> Option<ClientMessage> + Send + 'static,
) where
    S: futures_util::Stream<Item = Result<M, E>> + Unpin + Send + 'static,
    M: Send + 'static,
{
    tokio::spawn(async move {
        while let Some(Ok(msg)) = select! {
            msg = ws_receiver.next() => msg,
            _ = cancellation_token.cancelled() => None,
        } {
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
                        try_switch_protocol(&id, &text).unwrap_or_else(|e| {
                            println!("Client {} protocol switch error: {}", id, e);
                        });
                    } else if let Some(handler) = CLIENT_HANDLERS.get(&id) {
                        handler.handle_message(text.to_string());
                    } else {
                        println!("Client {} has no protocol handler", id);
                    }
                }
                ClientMessage::Close => break,
            }
        }
        println!("Client {} received ended", id);
        cancellation_token.cancel();
    });
}

fn try_switch_protocol(id: &ClientId, protocol_msg: &str) -> Result<(), String> {
    let parts: Vec<&str> = protocol_msg.split_whitespace().collect();
    if parts.len() == 2 {
        let protocol = Protocol::from_id(parts[1]).ok_or("Unknown protocol")?;
        if let Some(mut handler) = CLIENT_HANDLERS.get_mut(id) {
            *handler = protocol.get_handler(*id);
            println!("Client {} set protocol to {:?}", id, parts[1]);
            Ok(())
        } else {
            Err("Client handler not found".into())
        }
    } else {
        Err("Invalid protocol message format".into())
    }
}

pub fn try_send_to<T>(id: &ClientId, msg: T) -> Result<(), String>
where
    T: AsRef<str>,
{
    if let Some(sender) = CLIENT_SENDERS.get(id) {
        sender
            .send(msg.as_ref().into())
            .map_err(|e| format!("Failed to send message: {}", e))
    } else {
        Err("Sender not initialized".into())
    }
}

pub fn associate_player(id: &ClientId, username: &PlayerUsername) -> Result<(), String> {
    if CLIENT_TO_PLAYER.contains_key(id) {
        return Err("Player already logged in".into());
    }
    if PLAYER_TO_CLIENT.contains_key(username) {
        return Err("Player already logged in from another client".into());
    }
    CLIENT_TO_PLAYER.insert(*id, username.clone());
    PLAYER_TO_CLIENT.insert(username.clone(), *id);
    Ok(())
}

pub fn get_associated_player(id: &ClientId) -> Option<PlayerUsername> {
    CLIENT_TO_PLAYER.get(id).map(|entry| entry.value().clone())
}

pub fn get_associated_client(player: &PlayerUsername) -> Option<ClientId> {
    PLAYER_TO_CLIENT.get(player).map(|entry| *entry.value())
}

fn on_connect(id: &ClientId) {
    let _ = try_send_to(id, "Welcome!");
    let _ = try_send_to(id, "Login or Register");
    send_seeks_to(id);
}

pub fn try_protocol_broadcast(f: impl Fn(&BoxedProtocolHandler) -> ()) {
    for entry in CLIENT_HANDLERS.iter() {
        f(entry.value());
    }
}

pub fn get_protocol_handler(id: &ClientId) -> BoxedProtocolHandler {
    CLIENT_HANDLERS
        .get(id)
        .map(|entry| entry.value().clone_box())
        .unwrap_or(Box::new(ProtocolV2Handler::new(*id)))
}
