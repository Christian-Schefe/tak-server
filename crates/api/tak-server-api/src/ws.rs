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
use tak_domain::{GameId, UserId, chat::ChatConversation, game::GameRequest};
use tak_player_connection::{ConnectionId, PlayerSimpleConnectionPort};
use tak_server_api_contract::{
    game::{ForPlayerResponse, GameRequestResponse},
    ws::{
        ChatConversationResponse, ChatMessageResponse, ClientMessage, ClientMessageWrapper,
        ServerGameEventType, ServerMatchEventType, ServerMessage,
    },
};
use tak_server_app::{
    errors::{InternalError, OneOf},
    ports::notification::{ListenerGameMessageType, ListenerMatchEventType, ListenerMessage},
};
use tokio::select;
use tokio_util::sync::CancellationToken;
use unordered_pair::UnorderedPair;
use uuid::Uuid;

use crate::{
    AppState, ServiceError, auth::validate_jwt, game::from_metadata, seek::from_seek_view,
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
            tracing::error!("WebSocket receive task failed: {}", e);
        }
        if let Err(e) = send_res {
            tracing::error!("WebSocket send task failed: {}", e);
        }

        app.ws.remove_connection(conn_id);
        app.connection_driver.remove_connection(&conn_id).await;
        tracing::debug!("WebSocket connection {} handler finished", conn_id);
    })
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct MinimalClientMessage {
    response_id: Uuid,
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
                        tracing::debug!("Received WS message from {}: {:?}", connection_id, msg);
                        let response = if let Err(e) =
                            handle_client_message(&app, msg.message, connection_id).await
                        {
                            tracing::warn!("Failed to handle WS message: {}", e);
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
                        tracing::warn!("Failed to parse WS message: {}", e);
                        let response_id = match serde_json::from_str::<MinimalClientMessage>(&text)
                        {
                            Ok(minimal) => minimal.response_id,
                            Err(_) => Uuid::new_v4(),
                        };
                        let _ = sender.send(ServerMessage::Error {
                            message: "Invalid message format".to_string(),
                            code: 400,
                            response_id,
                        });
                    }
                }
            }
            Ok(axum::extract::ws::Message::Binary(bin)) => {
                tracing::debug!("Received WS binary message: {:?}", bin);
            }
            Ok(axum::extract::ws::Message::Close(frame)) => {
                tracing::debug!("WS connection closed: {:?}", frame);
                break;
            }
            Err(e) => {
                tracing::warn!("WS error: {}", e);
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
            .map_err(|e| {
                tracing::error!("Failed to send WS message: {}", e);
                ServiceError::from(InternalError)
            })?;
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
            let user_id = match validate_jwt(&token) {
                Some(id) => Ok(id),
                None => Err(ServiceError::Unauthorized(
                    "Invalid or expired authentication token".to_string(),
                )),
            }?;
            app.connection_driver
                .associate_connection(user_id, connection_id)
                .await;
            tracing::info!(
                "WS connection {} associated with account {}",
                connection_id,
                &user_id
            );
            Ok(())
        }
        _ => {
            let user_id = app
                .connection_driver
                .get_user_id(&connection_id)
                .ok_or_else(|| {
                    ServiceError::Unauthorized(
                        "WebSocket connection is not authenticated".to_string(),
                    )
                })?;
            handle_authenticated_client_message(app, user_id, msg, connection_id).await
        }
    }
}

async fn handle_authenticated_client_message(
    app: &AppState,
    user_id: UserId,
    msg: ClientMessage,
    connection_id: ConnectionId,
) -> Result<(), ServiceError> {
    match msg {
        ClientMessage::Authenticate { .. } => Err(ServiceError::BadRequest(
            "Already authenticated".to_string(),
        )),
        ClientMessage::GameAction { game_id, action } => {
            let Ok(game_id) = GameId::try_from(game_id) else {
                return Err(ServiceError::BadRequest(
                    "Invalid game ID format".to_string(),
                ));
            };
            tracing::info!("Received GameAction for game {}: {}", game_id, action);
            let Some(action) = action_from_ptn(&action) else {
                return Err(ServiceError::BadRequest(
                    "Invalid action format".to_string(),
                ));
            };
            app.app
                .gameplay_use_case
                .do_action(game_id, user_id, action)
                .await
                .map_err(OneOf::convert::<ServiceError>)?;
            Ok(())
        }
        ClientMessage::ChatMessage {
            message,
            conversation,
        } => {
            tracing::info!("Received ChatMessage: {:?} -> {}", conversation, message);
            let message_target = match conversation {
                ChatConversationResponse::Global => ChatConversation::Global,
                ChatConversationResponse::Room { room_name } => {
                    ChatConversation::Room { room_name }
                }
                ChatConversationResponse::Private { user_id1, user_id2 } => {
                    let Ok(user_id1) = UserId::try_from(user_id1) else {
                        return Err(ServiceError::BadRequest(
                            "Invalid user ID format in private conversation".to_string(),
                        ));
                    };
                    let Ok(user_id2) = UserId::try_from(user_id2) else {
                        return Err(ServiceError::BadRequest(
                            "Invalid user ID format in private conversation".to_string(),
                        ));
                    };
                    let members = UnorderedPair(user_id1, user_id2);
                    ChatConversation::Private { user_ids: members }
                }
            };
            app.app
                .chat_use_case
                .send_message(user_id, &message_target, &message)
                .await
                .map_err(OneOf::convert::<ServiceError>)?;
            Ok(())
        }
        ClientMessage::SpectateGame { game_id, spectate } => {
            tracing::info!("Received SpectateGame for game {}: {}", game_id, spectate);
            let Ok(game_id) = GameId::try_from(game_id) else {
                return Err(ServiceError::BadRequest(
                    "Invalid game ID format".to_string(),
                ));
            };
            if spectate {
                app.app
                    .gameplay_use_case
                    .observe_game(game_id, connection_id.0)
                    .await
                    .map_err(OneOf::convert::<ServiceError>)?;
            } else {
                app.app
                    .gameplay_use_case
                    .unobserve_game(game_id, connection_id.0);
            }
            Ok(())
        }
        ClientMessage::JoinChatRoom { room_name, join } => {
            tracing::info!("Received JoinChatRoom for room {}: {}", room_name, join);
            if join {
                app.app.chat_use_case.join_room(&room_name, connection_id.0);
            } else {
                app.app
                    .chat_use_case
                    .leave_room(&room_name, connection_id.0);
            }
            Ok(())
        }
    }
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
            if let Some(server_msg) = from_listener_message(message.clone()) {
                let _ = entry.sender.send(server_msg);
            }
        }
    }
}

fn from_listener_game_event_type(event_type: ListenerGameMessageType) -> ServerGameEventType {
    match event_type {
        ListenerGameMessageType::GameOver { game_result } => ServerGameEventType::GameEnded {
            result: game_result_to_string(game_result),
        },
        ListenerGameMessageType::GameAction {
            user_id: _,
            action,
            ply_index,
        } => ServerGameEventType::GameAction {
            ply_index,
            action: action_to_ptn(&action),
        },
        ListenerGameMessageType::GameActionUndone { ply_index } => {
            ServerGameEventType::GameActionUndone { ply_index }
        }
        ListenerGameMessageType::GameRequestChanged { user_id, request } => {
            ServerGameEventType::GameRequestChanged {
                user_id: user_id.0.to_string(),
                request: match request {
                    GameRequest::Draw(offer) => GameRequestResponse::Draw { offer },
                    GameRequest::Undo(request) => GameRequestResponse::Undo { request },
                    GameRequest::MoreTime(duration) => GameRequestResponse::MoreTime {
                        amount_ms: duration.map(|d| d.as_millis() as u64),
                    },
                },
            }
        }
    }
}

fn from_listener_message(message: ListenerMessage) -> Option<ServerMessage> {
    match message {
        ListenerMessage::SeekCreated { seek } => Some(ServerMessage::SeekCreated {
            seek: from_seek_view(seek),
        }),
        ListenerMessage::SeekCancelled { seek } | ListenerMessage::SeekAccepted { seek } => {
            Some(ServerMessage::SeekRemoved {
                seek_id: seek.id.to_string(),
            })
        }
        ListenerMessage::GameEvent {
            game_id,
            event_type,
            time_info,
        } => Some(ServerMessage::GameEvent {
            game_id: game_id.to_string(),
            event_type: from_listener_game_event_type(event_type),
            time_info: ForPlayerResponse {
                white: time_info.white_remaining.as_millis() as u64,
                black: time_info.black_remaining.as_millis() as u64,
            },
        }),
        ListenerMessage::GameStarted { game_id, game } => Some(ServerMessage::GameStarted {
            game: from_metadata(game_id, &game.metadata),
        }),
        ListenerMessage::GameEnded { game_id, .. } => Some(ServerMessage::GameEnded {
            game_id: game_id.to_string(),
        }),

        ListenerMessage::ChatMessage {
            message,
            conversation,
        } => {
            let conversation = match conversation {
                ChatConversation::Global => ChatConversationResponse::Global,
                ChatConversation::Room { room_name } => {
                    ChatConversationResponse::Room { room_name }
                }
                ChatConversation::Private { user_ids: members } => {
                    ChatConversationResponse::Private {
                        user_id1: members.0.to_string(),
                        user_id2: members.1.to_string(),
                    }
                }
            };
            Some(ServerMessage::ChatMessage {
                message: ChatMessageResponse {
                    message_id: message.id.0,
                    sender: message.sender.to_string(),
                    message: message.message,
                    timestamp: message.date,
                },
                conversation,
            })
        }
        ListenerMessage::MatchEvent {
            match_id,
            event_type,
        } => Some(ServerMessage::MatchEvent {
            match_id: match_id.to_string(),
            event_type: match event_type {
                ListenerMatchEventType::MatchReadinessChanged { user_id } => {
                    ServerMatchEventType::ReadinessChanged {
                        user_id: user_id.map(|id| id.to_string()),
                    }
                }
            },
        }),
        ListenerMessage::UsersOnline { users: accounts } => Some(ServerMessage::UsersOnline {
            user_ids: accounts.into_iter().map(|a| a.to_string()).collect(),
        }),
        ListenerMessage::ServerAlert { .. } => None,
    }
}
