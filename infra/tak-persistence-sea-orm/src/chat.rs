use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect,
};
use tak_persistence_sea_orm_entities::chat;
use tak_server_app::domain::{
    AccountId, ChatMessageId, RepoError,
    chat::{ChatConversation, ChatMessage, ChatRepository},
};

use crate::create_db_pool;

pub struct ChatRepositoryImpl {
    db: DatabaseConnection,
}

impl ChatRepositoryImpl {
    pub async fn new() -> Self {
        let db = create_db_pool().await;
        Self { db }
    }
}

fn conversation_id_from_chat_conversation(conversation: &ChatConversation) -> String {
    match conversation {
        ChatConversation::Private {
            account_ids: members,
        } => {
            let (first, second) = members.clone().into_ordered_tuple();
            format!("private:{}:{}", first, second)
        }
        ChatConversation::Room { room_name } => format!("room:{}", room_name),
        ChatConversation::Global => "global".to_string(),
    }
}

#[async_trait::async_trait]
impl ChatRepository for ChatRepositoryImpl {
    async fn save_message(
        &self,
        conversation: &ChatConversation,
        message: &ChatMessage,
    ) -> Result<(), RepoError> {
        let new_message = chat::ActiveModel {
            conversation: Set(conversation_id_from_chat_conversation(conversation)),
            id: Default::default(),
            from_account_id: Set(message.sender.0),
            date: Set(message.date),
            message: Set(message.message.to_string()),
        };
        new_message
            .insert(&self.db)
            .await
            .map_err(|e| RepoError::StorageError(e.to_string()))?;
        Ok(())
    }
    async fn get_messages(
        &self,
        conversation: &ChatConversation,
        before: Option<ChatMessageId>,
        limit: usize,
    ) -> Result<Vec<(ChatMessageId, ChatMessage)>, RepoError> {
        let mut query = chat::Entity::find().filter(
            chat::Column::Conversation.eq(conversation_id_from_chat_conversation(conversation)),
        );
        if let Some(before_id) = before {
            query = query.filter(chat::Column::Id.lt(before_id.0));
        }
        query = query.order_by_desc(chat::Column::Date).limit(limit as u64);

        let results = query
            .all(&self.db)
            .await
            .map_err(|e| RepoError::StorageError(e.to_string()))?;

        let messages = results
            .into_iter()
            .map(|model| {
                (
                    ChatMessageId::new(model.id),
                    ChatMessage {
                        date: model.date,
                        sender: AccountId(model.from_account_id),
                        message: model.message,
                    },
                )
            })
            .collect();

        Ok(messages)
    }
}
