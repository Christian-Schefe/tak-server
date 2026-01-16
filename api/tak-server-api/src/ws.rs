use std::sync::Arc;

use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::Response,
};
use dashmap::DashMap;
use futures::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use tak_core::ptn::{action_from_ptn, action_to_ptn, game_result_to_string};
use tak_player_connection::{ConnectionId, PlayerSimpleConnectionPort};
use tak_server_app::{
    domain::{AccountId, GameId, PlayerId},
    ports::notification::ListenerMessage,
};
use tokio::select;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    AppState, ServiceError,
    game::{ForPlayer, GameInfo},
    seek::SeekInfo,
};

pub async fn ws_handler(ws: WebSocketUpgrade, State(app): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| async move {
        let (ws_sender, ws_receiver) = socket.split();
        let cancellation_token = CancellationToken::new();
        let cancellation_token_clone = cancellation_token.clone();
        let conn_id = ConnectionId::new();
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
        app.connection_driver.remove_connection(&conn_id).await;
        log::info!("WebSocket connection {} handler finished", conn_id);
    })
}

async fn receive_ws(
    app: AppState,
    mut ws_receiver: SplitStream<WebSocket>,
    cancellation_token: CancellationToken,
    connection_id: ConnectionId,
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
    connection_id: ConnectionId,
) -> Result<(), ServiceError> {
    match msg {
        ClientMessage::Authenticate { token } => {
            let account_id = authenticate_ws_token(app, &token).await?;
            app.connection_driver
                .add_connection(&account_id, connection_id)
                .await;
            log::info!(
                "WS connection {} associated with account {}",
                connection_id,
                &account_id
            );
            Ok(())
        }
        _ => {
            let account_id = app
                .connection_driver
                .get_account_id(&connection_id)
                .ok_or_else(|| {
                    ServiceError::Unauthorized(
                        "WebSocket connection is not authenticated".to_string(),
                    )
                })?;
            let player_id = app
                .app
                .player_resolver_service
                .resolve_player_id_by_account_id(&account_id)
                .await
                .map_err(|_| {
                    ServiceError::Internal(
                        "Failed to resolve player ID for authenticated account".to_string(),
                    )
                })?;
            handle_authenticated_client_message(app, account_id, player_id, msg, connection_id)
                .await
        }
    }
}

async fn handle_authenticated_client_message(
    app: &AppState,
    _account_id: AccountId,
    player_id: PlayerId,
    msg: ClientMessage,
    _connection_id: ConnectionId,
) -> Result<(), ServiceError> {
    match msg {
        ClientMessage::Authenticate { .. } => Err(ServiceError::BadRequest(
            "Already authenticated".to_string(),
        )),
        ClientMessage::GameAction { game_id, action } => {
            log::info!("Received GameAction for game {}: {}", game_id, action);
            let Some(action) = action_from_ptn(&action) else {
                return Err(ServiceError::BadRequest(
                    "Invalid action format".to_string(),
                ));
            };
            if let Err(e) = app
                .app
                .game_do_action_use_case
                .do_action(GameId(game_id), player_id, action)
                .await
            {
                return Err(ServiceError::BadRequest(format!(
                    "Failed to perform game action: {:?}",
                    e
                )));
            }
            Ok(())
        }
    }
}

async fn authenticate_ws_token(app: &AppState, token: &str) -> Result<AccountId, ServiceError> {
    let account_id = app.auth.validate_account_jwt(token).ok_or_else(|| {
        ServiceError::Unauthorized("Invalid or expired authentication token".to_string())
    })?;
    Ok(account_id)
}

struct ConnectionEntry {
    cancellation_token: CancellationToken,
    sender: tokio::sync::mpsc::UnboundedSender<ServerMessage>,
}

pub struct WsService {
    connections: Arc<DashMap<ConnectionId, ConnectionEntry>>,
}

impl WsService {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
        }
    }

    fn add_connection(&self, id: ConnectionId, entry: ConnectionEntry) {
        self.connections.insert(id, entry);
    }

    fn remove_connection(&self, id: ConnectionId) {
        if let Some((_, entry)) = self.connections.remove(&id) {
            entry.cancellation_token.cancel();
        }
    }
}

impl PlayerSimpleConnectionPort for WsService {
    fn notify_connection(&self, connection_id: ConnectionId, message: &ListenerMessage) {
        if let Some(entry) = self.connections.get(&connection_id) {
            match ServerMessage::from_listener_message(message.clone()) {
                MessageTransformation::Transform(server_msg) => {
                    let _ = entry.sender.send(server_msg);
                }
                MessageTransformation::Ignore => {}
            }
        }
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
    GameAction { game_id: i64, action: String },
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
    GameAction {
        game_id: i64,
        ply_index: usize,
        action: String,
    },
    GameTimeUpdate {
        game_id: i64,
        remaining_ms: ForPlayer<u64>,
    },
    GameStarted {
        game: GameInfo,
    },
    GameEnded {
        game_id: i64,
        result: String,
    },
}

enum MessageTransformation {
    Ignore,
    Transform(ServerMessage),
}

impl ServerMessage {
    fn from_listener_message(message: ListenerMessage) -> MessageTransformation {
        match message {
            ListenerMessage::SeekCreated { seek } => {
                MessageTransformation::Transform(ServerMessage::SeekCreated {
                    seek: SeekInfo::from_seek_view(seek),
                })
            }
            ListenerMessage::SeekCanceled { seek } => {
                MessageTransformation::Transform(ServerMessage::SeekRemoved { seek_id: seek.id.0 })
            }
            ListenerMessage::GameAction {
                game_id,
                player_id: _,
                action,
            } => MessageTransformation::Transform(ServerMessage::GameAction {
                game_id: game_id.0,
                ply_index: action.ply_index,
                action: action_to_ptn(&action.action),
            }),
            ListenerMessage::GameStarted { game } => {
                MessageTransformation::Transform(ServerMessage::GameStarted {
                    game: GameInfo::from_ongoing_game_view(&game.metadata),
                })
            }
            ListenerMessage::GameEnded { game } => {
                MessageTransformation::Transform(ServerMessage::GameEnded {
                    game_id: game.metadata.id.0,
                    result: game_result_to_string(game.game.game_result()),
                })
            }
            ListenerMessage::GameTimeUpdate {
                game_id,
                white_time,
                black_time,
            } => MessageTransformation::Transform(ServerMessage::GameTimeUpdate {
                game_id: game_id.0,
                remaining_ms: ForPlayer {
                    white: white_time.as_millis() as u64,
                    black: black_time.as_millis() as u64,
                },
            }),
            _ => MessageTransformation::Ignore,
        }
    }
}
