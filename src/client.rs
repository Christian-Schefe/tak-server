use std::{
    sync::{Arc, LazyLock},
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
    game::send_games_to,
    player::PlayerUsername,
    protocol::{Protocol, ServerMessage, handle_client_message, handle_server_message},
    seek::send_seeks_to,
};

static CLIENT_SENDERS: LazyLock<
    Arc<DashMap<ClientId, (UnboundedSender<String>, CancellationToken)>>,
> = LazyLock::new(|| Arc::new(DashMap::new()));

static CLIENT_HANDLERS: LazyLock<Arc<DashMap<ClientId, Protocol>>> =
    LazyLock::new(|| Arc::new(DashMap::new()));

static CLIENT_TO_PLAYER: LazyLock<Arc<DashMap<ClientId, PlayerUsername>>> =
    LazyLock::new(|| Arc::new(DashMap::new()));

static PLAYER_TO_CLIENT: LazyLock<Arc<DashMap<PlayerUsername, ClientId>>> =
    LazyLock::new(|| Arc::new(DashMap::new()));

static LAST_ACTIVITY: LazyLock<Arc<DashMap<ClientId, Instant>>> =
    LazyLock::new(|| Arc::new(DashMap::new()));

pub type ClientId = Uuid;

pub enum ClientMessage {
    Text(String),
    Close,
}

fn new_client() -> ClientId {
    Uuid::new_v4()
}

fn on_disconnect(id: &ClientId) {
    CLIENT_SENDERS.remove(id);
    CLIENT_HANDLERS.remove(id);
    let player = CLIENT_TO_PLAYER.remove(id);
    if let Some((_, username)) = player {
        PLAYER_TO_CLIENT.remove(&username);
        update_online_players();
    }
}

pub fn handle_client_websocket(ws: WebSocket) {
    handle_client(
        ws,
        |s| Message::Binary(s.into()),
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

fn handle_send<S, M>(
    id: ClientId,
    mut ws_sender: impl SinkExt<M> + Unpin + Send + 'static,
    cancellation_token: CancellationToken,
    msg_factory: impl Fn(String) -> M + Send + 'static,
) where
    S: futures_util::Sink<M> + Unpin + Send + 'static,
    M: Send + 'static,
{
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    CLIENT_SENDERS.insert(id, (tx, cancellation_token.clone()));
    CLIENT_HANDLERS.insert(id, Protocol::V2);

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
        let _ = ws_sender.close().await;
        on_disconnect(&id);
        println!("Client {} send ended", id);
        cancellation_token.cancel();
    });
}

fn handle_receive<S, M, E>(
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
            LAST_ACTIVITY.insert(id, Instant::now());

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
                        handle_client_message(&handler, &id, text);
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

pub fn send_to<T>(id: &ClientId, msg: T)
where
    T: AsRef<str>,
{
    if let Err(e) = try_send_to(id, msg) {
        println!("Failed to send message to client {}: {}", id, e);
    }
}

pub fn try_send_to<T>(id: &ClientId, msg: T) -> Result<(), String>
where
    T: AsRef<str>,
{
    if let Some(sender) = CLIENT_SENDERS.get(id) {
        sender
            .0
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
    update_online_players();
    Ok(())
}

fn update_online_players() {
    let players: Vec<PlayerUsername> = CLIENT_TO_PLAYER
        .iter()
        .map(|entry| entry.value().clone())
        .collect();
    let msg = ServerMessage::PlayersOnline { players };
    try_protocol_broadcast(&msg);
}

pub fn get_associated_player(id: &ClientId) -> Option<PlayerUsername> {
    CLIENT_TO_PLAYER.get(id).map(|entry| entry.value().clone())
}

pub fn get_associated_client(player: &PlayerUsername) -> Option<ClientId> {
    PLAYER_TO_CLIENT.get(player).map(|entry| *entry.value())
}

fn on_connect(id: &ClientId) {
    send_to(id, "Welcome!");
    send_to(id, "Login or Register");
    send_seeks_to(id);
    send_games_to(id);
}

pub fn try_protocol_send(id: &ClientId, msg: &ServerMessage) {
    if let Some(handler) = CLIENT_HANDLERS.get(id) {
        handle_server_message(&handler, id, msg);
    }
}

pub fn try_protocol_multicast(ids: &[ClientId], msg: &ServerMessage) {
    for id in ids {
        try_protocol_send(id, msg);
    }
}

pub fn try_protocol_broadcast(msg: &ServerMessage) {
    for entry in CLIENT_HANDLERS.iter() {
        handle_server_message(entry.value(), entry.key(), msg);
    }
}

fn close_client(id: &ClientId) {
    let Some(cancellation_token) = CLIENT_SENDERS.get(id).map(|x| x.1.clone()) else {
        return;
    };
    cancellation_token.cancel();
}

pub fn launch_client_cleanup_task() {
    let timeout_duration = Duration::from_secs(300);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            let now = Instant::now();
            let inactive_clients: Vec<ClientId> = LAST_ACTIVITY
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
                close_client(&client_id);
            }
        }
    });
}
