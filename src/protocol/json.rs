use axum::{
    Router,
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};

use crate::{
    AppState, ArcChatService, ArcClientService, ArcGameService, ArcPlayerService, ServiceError,
    ServiceResult,
    client::ClientId,
    game::GameId,
    player::PlayerUsername,
    protocol::{ChatMessageSource, DisconnectReason, ServerGameMessage, ServerMessage},
    seek::SeekId,
};
use tak_core::ptn::{action_to_ptn, game_state_to_string};

mod auth;
mod chat;
mod game;
mod game_list;
mod seek;
mod sudo;

pub struct ProtocolJsonHandler {
    client_service: ArcClientService,
    player_service: ArcPlayerService,
    game_service: ArcGameService,
    chat_service: ArcChatService,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ClientMessage {
    Ping,
    Login {
        token: String,
    },
    LoginGuest {
        token: Option<String>,
    },
    GameAction {
        game_id: GameId,
        action: String,
    },
    RequestUndo {
        game_id: GameId,
        request: bool,
    },
    OfferDraw {
        game_id: GameId,
        offer: bool,
    },
    Resign {
        game_id: GameId,
    },
    ChatMessage {
        message: String,
        room: Option<String>,
        player: Option<PlayerUsername>,
    },
    ObserveGame {
        game_id: GameId,
        observe: bool,
    },
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackedClientMessage {
    pub msg_id: Option<String>,
    #[serde(flatten)]
    pub msg: ClientMessage,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ClientResponse {
    Error { message: String },
    Ok,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackedClientResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msg_id: Option<String>,
    #[serde(flatten)]
    pub response: ClientResponse,
}

impl ProtocolJsonHandler {
    pub fn new(
        client_service: ArcClientService,
        player_service: ArcPlayerService,
        game_service: ArcGameService,
        chat_service: ArcChatService,
    ) -> Self {
        Self {
            client_service,
            player_service,
            game_service,
            chat_service,
        }
    }

    pub fn handle_server_message(&self, id: &ClientId, msg: &ServerMessage) {
        let msg = server_message_to_json(msg);
        self.send_json_to(id, &msg);
    }

    pub fn send_json_to(&self, id: &ClientId, msg: &impl serde::Serialize) {
        match serde_json::to_string(msg) {
            Ok(json) => crate::client::send_to(&**self.client_service, id, &json),
            Err(e) => eprintln!(
                "Failed to serialize message to JSON for client {}: {}",
                id, e
            ),
        }
    }

    pub fn handle_client_message(&self, id: &ClientId, msg: String) {
        if msg.to_ascii_lowercase().starts_with("protocol") {
            return;
        }
        let msg = match serde_json::from_str::<TrackedClientMessage>(&msg) {
            Ok(msg) => msg,
            Err(e) => {
                println!("Failed to parse JSON message from client {}: {}", id, e);
                self.send_json_to(
                    id,
                    &ClientResponse::Error {
                        message: "Invalid JSON".to_string(),
                    },
                );
                return;
            }
        };
        let response: ServiceResult<ClientResponse> = match msg.msg {
            ClientMessage::Ping => Ok(ClientResponse::Ok),
            ClientMessage::Login { token } => self.handle_login_message(id, &token),
            ClientMessage::LoginGuest { token } => {
                self.handle_login_guest_message(id, token.as_deref())
            }
            msg => self.handle_logged_in_client_message(id, msg),
        };
        let tracked_response = TrackedClientResponse {
            msg_id: msg.msg_id,
            response: response.unwrap_or_else(|e| ClientResponse::Error {
                message: e.to_string(),
            }),
        };
        self.send_json_to(id, &tracked_response);
    }

    fn handle_logged_in_client_message(
        &self,
        id: &ClientId,
        msg: ClientMessage,
    ) -> ServiceResult<ClientResponse> {
        let Some(username) = self.client_service.get_associated_player(id) else {
            return ServiceError::unauthorized("Client not logged in");
        };
        match msg {
            ClientMessage::GameAction { game_id, action } => {
                self.handle_game_action(&username, &game_id, &action)
            }
            ClientMessage::RequestUndo { game_id, request } => {
                self.handle_undo_request_message(&username, &game_id, request)
            }
            ClientMessage::OfferDraw { game_id, offer } => {
                self.handle_draw_offer_message(&username, &game_id, offer)
            }
            ClientMessage::Resign { game_id } => self.handle_resign_message(&username, &game_id),
            ClientMessage::ChatMessage {
                message,
                room,
                player,
            } => self.handle_chat_message(&username, &message, &room, &player),

            ClientMessage::Ping
            | ClientMessage::Login { .. }
            | ClientMessage::LoginGuest { .. } => ServiceError::internal("Unhandled message type"),
            ClientMessage::ObserveGame { game_id, observe } => {
                self.handle_observe_game_message(id, &game_id, observe)
            }
        }
    }
}

pub fn register_http_endpoints() -> Router<AppState> {
    Router::new()
        .route("/seeks", post(seek::handle_add_seek_endpoint))
        .route("/seeks", delete(seek::handle_remove_seek_endpoint))
        .route("/seeks", get(seek::get_seeks_endpoint))
        .route("/seeks/{id}", get(seek::get_seek_endpoint))
        .route("/seeks/{id}/accept", get(seek::accept_seek_endpoint))
        .route("/games", get(game_list::get_game_ids_endpoint))
        .route("/games/{id}", get(game_list::get_game_endpoint))
        .route(
            "/auth/request-password-reset",
            post(auth::request_password_reset_endpoint),
        )
        .route("/auth/reset-password", post(auth::reset_password_endpoint))
        .route(
            "/auth/change-password",
            post(auth::change_password_endpoint),
        )
        .route("/sudo/ban", post(sudo::sudo_ban_endpoint))
        .route("/sudo/unban", post(sudo::sudo_unban_endpoint))
        .route("/sudo/set-admin", post(sudo::sudo_admin_endpoint))
        .route("/sudo/set-mod", post(sudo::sudo_mod_endpoint))
        .route("/sudo/set-bot", post(sudo::sudo_bot_endpoint))
        .route("/sudo/set-gag", post(sudo::sudo_gag_endpoint))
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum JsonServerMessage {
    SeekList {
        add: bool,
        seek_id: SeekId,
    },
    GameList {
        add: bool,
        game_id: GameId,
    },
    GameStart {
        game_id: GameId,
    },
    GameMessage {
        game_id: GameId,
        message: JsonServerGameMessage,
    },
    PlayersOnline {
        players: Vec<String>,
    },
    ChatMessage {
        from: PlayerUsername,
        message: String,
        source: JsonChatMessageSource,
    },
    RoomMembership {
        room: String,
        joined: bool,
    },
    AcceptRematch {
        seek_id: SeekId,
    },
    ConnectionClosed {
        reason: String,
    },
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum JsonServerGameMessage {
    Action { action: String },
    TimeUpdate { white_ms: u64, black_ms: u64 },
    Undo,
    GameOver { game_state: String },
    UndoRequest { request: bool },
    DrawOffer { offer: bool },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum JsonChatMessageSource {
    Global,
    Room { room_name: String },
    Private,
}

fn server_message_to_json(msg: &ServerMessage) -> JsonServerMessage {
    match msg {
        ServerMessage::SeekList { add, seek } => JsonServerMessage::SeekList {
            add: *add,
            seek_id: seek.id,
        },
        ServerMessage::GameList { add, game } => JsonServerMessage::GameList {
            add: *add,
            game_id: game.id,
        },
        ServerMessage::GameStart { game_id } => JsonServerMessage::GameStart { game_id: *game_id },
        ServerMessage::GameMessage { game_id, message } => JsonServerMessage::GameMessage {
            game_id: *game_id,
            message: server_game_message_to_json(message),
        },
        ServerMessage::PlayersOnline { players } => JsonServerMessage::PlayersOnline {
            players: players.clone(),
        },
        ServerMessage::ChatMessage {
            from,
            message,
            source,
        } => JsonServerMessage::ChatMessage {
            from: from.clone(),
            message: message.clone(),
            source: match source {
                ChatMessageSource::Global => JsonChatMessageSource::Global,
                ChatMessageSource::Room { name } => JsonChatMessageSource::Room {
                    room_name: name.clone(),
                },
                ChatMessageSource::Private => JsonChatMessageSource::Private,
            },
        },
        ServerMessage::RoomMembership { room, joined } => JsonServerMessage::RoomMembership {
            room: room.clone(),
            joined: *joined,
        },
        ServerMessage::AcceptRematch { seek_id } => {
            JsonServerMessage::AcceptRematch { seek_id: *seek_id }
        }
        ServerMessage::ConnectionClosed { reason } => JsonServerMessage::ConnectionClosed {
            reason: match reason {
                DisconnectReason::NewSession => "New session from another client".to_string(),
                DisconnectReason::Inactivity => "Disconnected due to inactivity".to_string(),
            },
        },
    }
}

fn server_game_message_to_json(msg: &ServerGameMessage) -> JsonServerGameMessage {
    match msg {
        ServerGameMessage::Action { action } => JsonServerGameMessage::Action {
            action: action_to_ptn(action),
        },
        ServerGameMessage::TimeUpdate {
            remaining_white,
            remaining_black,
        } => JsonServerGameMessage::TimeUpdate {
            white_ms: remaining_white.as_millis() as u64,
            black_ms: remaining_black.as_millis() as u64,
        },
        ServerGameMessage::Undo => JsonServerGameMessage::Undo,
        ServerGameMessage::GameOver { game_state } => JsonServerGameMessage::GameOver {
            game_state: game_state_to_string(game_state),
        },
        ServerGameMessage::UndoRequest { request } => {
            JsonServerGameMessage::UndoRequest { request: *request }
        }
        ServerGameMessage::DrawOffer { offer } => {
            JsonServerGameMessage::DrawOffer { offer: *offer }
        }
    }
}
