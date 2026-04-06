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
use tak_server_api_contract::{
    game::{ForPlayer, JsonGameRequestType},
    ws::{
        ClientMessage, ClientMessageWrapper, JsonChatConversation, JsonChatMessage,
        ServerGameEventType, ServerMatchEventType, ServerMessage,
    },
};
use tak_server_app::{
    domain::{AccountId, GameId, PlayerId, chat::ChatConversation, game::request::GameRequestType},
    ports::notification::{ListenerGameMessageType, ListenerMatchEventType, ListenerMessage},
    workflow::{
        chat::message::ChatSendMessageError,
        gameplay::{
            do_action::{ActionResult, DoActionError, PlayerActionError},
            observe::ObserveGameError,
        },
    },
};
use tokio::select;
use tokio_util::sync::CancellationToken;
use unordered_pair::UnorderedPair;
use uuid::Uuid;

use crate::{AppState, ServiceError, game::from_metadata_view, seek::from_seek_view};

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
        tracing::info!("WebSocket connection {} handler finished", conn_id);
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
                        tracing::info!("Received WS message from {}: {:?}", connection_id, msg);
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
                        let _ = sender.send(ServerMessage::Error {
                            message: "Invalid message format".to_string(),
                            code: 400,
                            response_id: Uuid::new_v4(),
                        });
                    }
                }
            }
            Ok(axum::extract::ws::Message::Binary(bin)) => {
                tracing::info!("Received WS binary message: {:?}", bin);
            }
            Ok(axum::extract::ws::Message::Close(frame)) => {
                tracing::info!("WS connection closed: {:?}", frame);
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
            tracing::info!(
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
    account_id: AccountId,
    player_id: PlayerId,
    msg: ClientMessage,
    connection_id: ConnectionId,
) -> Result<(), ServiceError> {
    match msg {
        ClientMessage::Authenticate { .. } => Err(ServiceError::BadRequest(
            "Already authenticated".to_string(),
        )),
        ClientMessage::GameAction { game_id, action } => {
            tracing::info!("Received GameAction for game {}: {}", game_id, action);
            let Some(action) = action_from_ptn(&action) else {
                return Err(ServiceError::BadRequest(
                    "Invalid action format".to_string(),
                ));
            };
            match app
                .app
                .game_do_action_use_case
                .do_action(GameId(game_id), player_id, action)
                .await
            {
                ActionResult::Success => Ok(()),
                ActionResult::ActionError(e) => match e {
                    DoActionError::InvalidAction(reason) => Err(ServiceError::BadRequest(format!(
                        "Invalid action: {:?}",
                        reason
                    ))),
                    DoActionError::NotPlayersTurn => {
                        Err(ServiceError::BadRequest("Not player's turn".to_string()))
                    }
                },
                ActionResult::NotPossible(e) => match e {
                    PlayerActionError::GameNotFound => {
                        Err(ServiceError::BadRequest("Game not found".to_string()))
                    }
                    PlayerActionError::NotAPlayerInGame => {
                        Err(ServiceError::BadRequest("Not a player in game".to_string()))
                    }
                },
            }
        }
        ClientMessage::ChatMessage {
            message,
            conversation,
        } => {
            tracing::info!("Received ChatMessage: {:?} -> {}", conversation, message);
            let message_target = match conversation {
                JsonChatConversation::Global => ChatConversation::Global,
                JsonChatConversation::Room { room_name } => ChatConversation::Room { room_name },
                JsonChatConversation::Private {
                    account_id1,
                    account_id2,
                } => {
                    let Some(account_id1) = AccountId::from_string(account_id1) else {
                        return Err(ServiceError::BadRequest(
                            "Invalid account ID format in private conversation".to_string(),
                        ));
                    };
                    let Some(account_id2) = AccountId::from_string(account_id2) else {
                        return Err(ServiceError::BadRequest(
                            "Invalid account ID format in private conversation".to_string(),
                        ));
                    };
                    let members = UnorderedPair(account_id1, account_id2);
                    ChatConversation::Private {
                        account_ids: members,
                    }
                }
            };
            match app
                .app
                .chat_message_use_case
                .send_message(&account_id, &message_target, &message)
                .await
            {
                Ok(_) => Ok(()),
                Err(e) => match e {
                    ChatSendMessageError::NotAllowed(reason) => {
                        Err(ServiceError::BadRequest(reason))
                    }
                    ChatSendMessageError::RepositoryError => Err(ServiceError::Internal(
                        "Failed to save chat message".to_string(),
                    )),
                },
            }
        }
        ClientMessage::SpectateGame { game_id, spectate } => {
            tracing::info!("Received SpectateGame for game {}: {}", game_id, spectate);
            if spectate {
                app.app
                    .game_observe_use_case
                    .observe_game(GameId(game_id), connection_id.0)
                    .map_err(|e| match e {
                        ObserveGameError::GameNotFound => {
                            ServiceError::NotFound("Game not found".to_string())
                        }
                    })?;
            } else {
                app.app
                    .game_observe_use_case
                    .unobserve_game(GameId(game_id), connection_id.0);
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
            match from_listener_message(message.clone()) {
                MessageTransformation::Transform(server_msg) => {
                    let _ = entry.sender.send(server_msg);
                }
                MessageTransformation::Ignore => {}
            }
        }
    }
}

fn from_listener_game_event_type(
    event_type: ListenerGameMessageType,
) -> Option<ServerGameEventType> {
    let res = match event_type {
        ListenerGameMessageType::GameOver { game_result } => ServerGameEventType::GameEnded {
            result: game_result_to_string(&game_result),
        },
        ListenerGameMessageType::GameAction {
            player_id: _,
            action,
            ply_index,
        } => ServerGameEventType::GameAction {
            ply_index,
            action: action_to_ptn(&action),
        },
        ListenerGameMessageType::GameActionUndone { ply_index } => {
            ServerGameEventType::GameActionUndone { ply_index }
        }
        ListenerGameMessageType::GameRequestAdded {
            requesting_player_id,
            request,
        } => {
            let request_type = match request.request_type {
                GameRequestType::Draw => JsonGameRequestType::Draw,
                GameRequestType::Undo => JsonGameRequestType::Undo,
                GameRequestType::MoreTime(_) => return None,
            };
            ServerGameEventType::GameRequestAdded {
                request_id: request.id.0,
                request_type,
                from_player_id: requesting_player_id.0.to_string(),
            }
        }
        ListenerGameMessageType::GameRequestRetracted {
            retracting_player_id: _,
            request,
        } => ServerGameEventType::GameRequestRemoved {
            request_id: request.id.0,
        },
        ListenerGameMessageType::GameRequestRejected {
            rejecting_player_id: _,
            request,
        } => ServerGameEventType::GameRequestRemoved {
            request_id: request.id.0,
        },
        ListenerGameMessageType::GameRequestAccepted {
            accepting_player_id: _,
            request,
        } => ServerGameEventType::GameRequestRemoved {
            request_id: request.id.0,
        },
    };
    Some(res)
}

enum MessageTransformation {
    Ignore,
    Transform(ServerMessage),
}

fn from_listener_message(message: ListenerMessage) -> MessageTransformation {
    match message {
        ListenerMessage::SeekCreated { seek } => {
            MessageTransformation::Transform(ServerMessage::SeekCreated {
                seek: from_seek_view(seek),
            })
        }
        ListenerMessage::SeekCanceled { seek } | ListenerMessage::SeekAccepted { seek } => {
            MessageTransformation::Transform(ServerMessage::SeekRemoved { seek_id: seek.id.0 })
        }
        ListenerMessage::GameEvent {
            game_id,
            event_type,
            time_info,
        } => from_listener_game_event_type(event_type).map_or(
            MessageTransformation::Ignore,
            |event| {
                MessageTransformation::Transform(ServerMessage::GameEvent {
                    game_id: game_id.0,
                    event_type: event,
                    time_info: ForPlayer {
                        white: time_info.white_remaining.as_millis() as u64,
                        black: time_info.black_remaining.as_millis() as u64,
                    },
                })
            },
        ),
        ListenerMessage::GameStarted { game } => {
            MessageTransformation::Transform(ServerMessage::GameStarted {
                game: from_metadata_view(game.id, &game.metadata),
            })
        }
        ListenerMessage::GameEnded { game } => {
            MessageTransformation::Transform(ServerMessage::GameEnded { game_id: game.id.0 })
        }

        ListenerMessage::ChatMessage {
            message,
            conversation,
        } => {
            let conversation = match conversation {
                ChatConversation::Global => JsonChatConversation::Global,
                ChatConversation::Room { room_name } => JsonChatConversation::Room { room_name },
                ChatConversation::Private {
                    account_ids: members,
                } => JsonChatConversation::Private {
                    account_id1: members.0.to_string(),
                    account_id2: members.1.to_string(),
                },
            };
            MessageTransformation::Transform(ServerMessage::ChatMessage {
                message: JsonChatMessage {
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
        } => MessageTransformation::Transform(ServerMessage::MatchEvent {
            match_id: match_id.0,
            event_type: match event_type {
                ListenerMatchEventType::MatchRematchRequestAdded {
                    requesting_player_id,
                } => ServerMatchEventType::MatchRematchRequestAdded {
                    from_player_id: requesting_player_id.to_string(),
                },
                ListenerMatchEventType::MatchRematchRequestRemoved => {
                    ServerMatchEventType::MatchRematchRequestRemoved {}
                }
            },
        }),
        ListenerMessage::AccountsOnline { .. } => MessageTransformation::Ignore,
        ListenerMessage::ServerAlert { .. } => MessageTransformation::Ignore,
    }
}
