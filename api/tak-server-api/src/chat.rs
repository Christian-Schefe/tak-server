use axum::{
    Json,
    extract::{Path, Query, State},
};
use tak_server_api_contract::ws::JsonChatMessage;
use tak_server_app::domain::{AccountId, ChatMessageId, chat::ChatConversation};
use unordered_pair::UnorderedPair;

use crate::{AppState, ServiceError};

pub fn register_routes(router: axum::Router<AppState>) -> axum::Router<AppState> {
    router.route(
        "/chat/{conversation_id}",
        axum::routing::get(get_chat_messages),
    )
}

#[derive(serde::Deserialize)]
pub struct MessagesQuery {
    pub before: Option<i64>,
    pub limit: usize,
}

pub async fn get_chat_messages(
    State(app): State<AppState>,
    Path(conversation_id): Path<String>,
    Query(messages_query): Query<MessagesQuery>,
) -> Result<Json<Vec<JsonChatMessage>>, ServiceError> {
    let parts = conversation_id
        .split(':')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    let Some(first) = parts.first() else {
        return Err(ServiceError::BadRequest(
            "Conversation ID cannot be empty".to_string(),
        ));
    };
    let conversation = match *first {
        "global" => ChatConversation::Global,
        "room" => {
            if parts.len() != 2 {
                return Err(ServiceError::BadRequest(
                    "Invalid room conversation ID format".to_string(),
                ));
            }
            let room_name = parts[1].to_string();
            ChatConversation::Room { room_name }
        }
        "private" => {
            if parts.len() != 3 {
                return Err(ServiceError::BadRequest(
                    "Invalid private conversation ID format".to_string(),
                ));
            }
            let account_ids = UnorderedPair(
                AccountId::from_string(parts[1].to_string()).ok_or_else(|| {
                    ServiceError::BadRequest("Invalid account ID in conversation ID".to_string())
                })?,
                AccountId::from_string(parts[2].to_string()).ok_or_else(|| {
                    ServiceError::BadRequest("Invalid account ID in conversation ID".to_string())
                })?,
            );
            ChatConversation::Private { account_ids }
        }
        _ => {
            return Err(ServiceError::BadRequest(
                "Invalid conversation ID format".to_string(),
            ));
        }
    };
    let messages = app
        .app
        .chat_message_use_case
        .get_messages(
            &conversation,
            messages_query.before.map(|v| ChatMessageId(v)),
            messages_query.limit,
        )
        .await
        .map_err(|_| ServiceError::Internal("Failed to retrieve chat messages".to_string()))?;

    let messages = messages
        .into_iter()
        .map(|msg| JsonChatMessage {
            message_id: msg.id.0,
            sender: msg.sender.to_string(),
            message: msg.message,
            timestamp: msg.date,
        })
        .collect();
    Ok(Json(messages))
}
