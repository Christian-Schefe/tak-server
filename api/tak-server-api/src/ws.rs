use std::{collections::{HashMap, HashSet}, sync::Arc};

use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::Response,
};
use futures::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use more_concurrent_maps::bijection::BiMap;
use parking_lot::RwLock;
use tak_server_app::{
    domain::{AccountId, ListenerId, PlayerId},
    ports::{
        connection::AccountConnectionPort,
        notification::{ListenerMessage, ListenerNotificationPort},
    },
};
use tokio::select;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{AppState, ServiceError, seek::SeekInfo};

pub async fn ws_handler(ws: WebSocketUpgrade, State(app): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| async move {
        let (ws_sender, ws_receiver) = socket.split();
        let cancellation_token = CancellationToken::new();
        let cancellation_token_clone = cancellation_token.clone();
        let conn_id = Uuid::new_v4();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let tx_clone = tx.clone();
        let app_clone = app.clone();
        let receive_task = tokio::spawn(receive_ws(
            app_clone,
            ws_receiver,
            cancellation_token_clone,
            conn_id,
            tx_clone,
        ));
        let cancellation_token_clone = cancellation_token.clone();
        let send_task = tokio::spawn(send_ws(ws_sender, rx, cancellation_token_clone));

        let entry = ConnectionEntry {
            cancellation_token: cancellation_token.clone(),
            sender: tx,
        };

        app.ws.add_connection(conn_id, entry);

        let (receive_res, send_res) = tokio::join!(receive_task, send_task);
        if let Err(e) = receive_res {
            log::error!("WebSocket receive task failed: {}", e);
        }
        if let Err(e) = send_res {
            log::error!("WebSocket send task failed: {}", e);
        }

        app.ws.remove_connection(conn_id);
    })
}

async fn receive_ws(
    app: AppState,
    mut ws_receiver: SplitStream<WebSocket>,
    cancellation_token: CancellationToken,
    connection_id: Uuid,
    sender: tokio::sync::mpsc::UnboundedSender<ServerMessage>,
) {
    while let Some(msg) = select! {
        _ = cancellation_token.cancelled() => None,
        msg = ws_receiver.next() => msg,
    } {
        match msg {
            Ok(axum::extract::ws::Message::Text(text)) => {
                match serde_json::from_str::<ClientMessageWrapper>(&text) {
                    Ok(msg) => {
                        log::info!("Received WS message from {}: {:?}", connection_id, msg);
                        let response = if let Err(e) =
                            handle_client_message(&app, msg.message, connection_id).await
                        {
                            log::error!("Failed to handle WS message: {}", e);
                            ServerMessage::Error {
                                message: e.to_string(),
                                code: e.status_code().as_u16(),
                                response_id: msg.response_id,
                            }
                        } else {
                            ServerMessage::Success {
                                response_id: msg.response_id,
                            }
                        };
                        let _ = sender.send(response);
                    }
                    Err(e) => {
                        log::error!("Failed to parse WS message: {}", e);
                    }
                }
            }
            Ok(axum::extract::ws::Message::Binary(bin)) => {
                log::info!("Received WS binary message: {:?}", bin);
            }
            Ok(axum::extract::ws::Message::Close(frame)) => {
                log::info!("WS connection closed: {:?}", frame);
                break;
            }
            Err(e) => {
                log::error!("WS error: {}", e);
                break;
            }
            _ => {}
        }
    }
    cancellation_token.cancel();
}

async fn send_ws(
    mut ws_sender: SplitSink<WebSocket, Message>,
    channel: tokio::sync::mpsc::UnboundedReceiver<ServerMessage>,
    cancellation_token: CancellationToken,
) -> Result<(), ServiceError> {
    let mut channel = channel;
    while let Some(msg) = select! {
        _ = cancellation_token.cancelled() => None,
        msg = channel.recv() => msg,
    } {
        ws_sender
            .send(Message::Text(serde_json::to_string(&msg).unwrap().into()))
            .await
            .map_err(|e| ServiceError::Internal(format!("Failed to send WS message: {}", e)))?;
    }
    cancellation_token.cancel();
    Ok(())
}

async fn handle_client_message(
    app: &AppState,
    msg: ClientMessage,
    connection_id: Uuid,
) -> Result<(), ServiceError> {
    match msg {
        ClientMessage::Authenticate { token } => {
            let player_id = authenticate_ws_token(app, &token).await?;
            app.ws.set_connection_owner(connection_id, player_id);
            Ok(())
        }
    }
}

async fn authenticate_ws_token(app: &AppState, token: &str) -> Result<PlayerId, ServiceError> {
    let account_id = app.auth.validate_account_jwt(token).ok_or_else(|| {
        ServiceError::Unauthorized("Invalid or expired authentication token".to_string())
    })?;
    let player_id = app
        .app
        .player_resolver_service
        .resolve_player_id_by_account_id(&account_id)
        .await
        .map_err(|_| {
            ServiceError::Internal("Failed to resolve player ID for account".to_string())
        })?;
    Ok(player_id)
}

struct ConnectionEntry {
    cancellation_token: CancellationToken,
    sender: tokio::sync::mpsc::UnboundedSender<ServerMessage>,
}

struct ConnectionRegistry {
    connections: HashMap<Uuid, ConnectionEntry>,
    listener_to_connection: HashMap<ListenerId, HashSet<Uuid>>,
    connection_to_listeners: HashMap<Uuid, ListenerId>,
    listener_player_map: BiMap<ListenerId, PlayerId>,
}

pub struct WsService {
    registry: Arc<RwLock<ConnectionRegistry>>,
}

impl WsService {
    pub fn new() -> Self {
        Self {
            registry: Arc::new(RwLock::new(ConnectionRegistry {
                connections: HashMap::new(),
                listener_to_connection: HashMap::new(),
                connection_to_listeners: HashMap::new(),
                listener_player_map: BiMap::new(),
            })),
        }
    }

    fn add_connection(&self, id: Uuid, entry: ConnectionEntry) {
        let mut registry = self.registry.write();
        registry.connections.insert(id, entry);
    }

    fn remove_connection(&self, id: Uuid) {
        let mut registry = self.registry.write();
        if let Some(entry) = registry.connections.remove(&id) {
            entry.cancellation_token.cancel();
        }
    }

    fn set_connection_owner(&self, connection_id: Uuid, player: PlayerId) {
        let mut registry = self.registry.write();
        if let Some(old_listener_id) = registry
            .connection_to_listeners
            .get(&connection_id)
            .copied()
        {
            if let Some(connections) = registry.listener_to_connection.get_mut(&old_listener_id) {
                connections.remove(&connection_id);
                if connections.is_empty() {
                    registry.listener_to_connection.remove(&old_listener_id);
                    registry
                        .listener_player_map
                        .remove_by_left(&old_listener_id);
                }
            }
        }
        let listener_id = if let Some(existing_listener_id) =
            registry.listener_player_map.get_by_right(&player)
        {
            *existing_listener_id
        } else {
            let new_listener_id = ListenerId::new();
            registry
                .listener_player_map
                .try_insert(new_listener_id, player);
            new_listener_id
        };
        registry
            .listener_to_connection
            .entry(listener_id)
            .or_insert_with(HashSet::new)
            .insert(connection_id);
        registry
            .connection_to_listeners
            .insert(connection_id, listener_id);
    }
}


#[derive(serde::Deserialize, Debug)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ClientMessage {
    Authenticate { token: String },
}

#[derive(serde::Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
pub struct ClientMessageWrapper {
    #[serde(flatten)]
    pub message: ClientMessage,
    pub response_id: Uuid,
}

#[derive(serde::Serialize, Debug, Clone)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ServerMessage {
    Success {
        response_id: Uuid,
    },
    Error {
        message: String,
        code: u16,
        response_id: Uuid,
    },
    SeekCreated {
        seek: SeekInfo,
    },
    SeekRemoved {
        seek_id: u64,
    },
}

impl ServerMessage {
    pub fn from_listener_message(message: ListenerMessage) -> Option<Self> {
        match message {
            ListenerMessage::SeekCreated { seek } => Some(ServerMessage::SeekCreated {
                seek: SeekInfo::from_seek_view(seek),
            }),
            ListenerMessage::SeekCanceled { seek } => {
                Some(ServerMessage::SeekRemoved { seek_id: seek.id.0 })
            }
            _ => None,
        }
    }
}
