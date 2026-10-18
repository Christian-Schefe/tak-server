use sea_orm::{ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use tak_domain::{
    ChatMessageId, UserId,
    chat::{ChatConversation, ChatMessage},
};
use tak_persistence_sea_orm_entities::chat;
use tak_server_app::ports::{
    chat::ChatRepository,
    transaction::{TransactionError, TransactionItem},
};

use crate::transaction::TransactionContext;

pub struct ChatRepositoryImpl;

fn conversation_id_from_chat_conversation(conversation: &ChatConversation) -> String {
    match conversation {
        ChatConversation::Private { user_ids: members } => {
            let (first, second) = members.clone().into_ordered_tuple();
            format!("private:{}:{}", first, second)
        }
        ChatConversation::Room { room_name } => format!("room:{}", room_name),
        ChatConversation::Global => "global".to_string(),
    }
}

impl ChatRepository<TransactionContext> for ChatRepositoryImpl {
    fn save_message(
        &self,
        conversation: ChatConversation,
        message: ChatMessage,
    ) -> TransactionItem<TransactionContext, Result<ChatMessageId, TransactionError>> {
        TransactionItem::new(Box::new(move |ctx| {
            Box::pin(async move {
                let new_message = chat::ActiveModel {
                    conversation: Set(conversation_id_from_chat_conversation(&conversation)),
                    id: Default::default(),
                    from_user_id: Set(message.sender.0),
                    date: Set(message.date),
                    message: Set(message.message.to_string()),
                };
                let inserted_message = chat::Entity::insert(new_message)
                    .exec(ctx)
                    .await
                    .map_err(|e| TransactionError(e.to_string()))?;
                Ok(ChatMessageId::new(inserted_message.last_insert_id))
            })
        }))
    }

    fn get_messages(
        &self,
        conversation: ChatConversation,
        cursor: Option<ChatMessageId>,
        limit: usize,
    ) -> TransactionItem<
        TransactionContext,
        Result<Vec<(ChatMessageId, ChatMessage)>, TransactionError>,
    > {
        TransactionItem::new(Box::new(move |ctx| {
            Box::pin(async move {
                let mut query = chat::Entity::find().filter(
                    chat::Column::Conversation
                        .eq(conversation_id_from_chat_conversation(&conversation)),
                );
                if let Some(before_id) = cursor {
                    query = query.filter(chat::Column::Id.lt(before_id.0));
                }
                query = query.order_by_desc(chat::Column::Date).limit(limit as u64);

                let results = query
                    .all(ctx)
                    .await
                    .map_err(|e| TransactionError(e.to_string()))?;

                let messages = results
                    .into_iter()
                    .map(|model| {
                        (
                            ChatMessageId::new(model.id),
                            ChatMessage {
                                date: model.date,
                                sender: UserId(model.from_user_id),
                                message: model.message,
                            },
                        )
                    })
                    .collect();

                Ok(messages)
            })
        }))
    }
}
