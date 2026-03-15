use chrono::Utc;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "chat_messages")]
pub struct Model {
    #[sea_orm(unique_key = "unique_message")]
    pub conversation: String,
    #[sea_orm(primary_key, auto_increment = true, unique_key = "unique_message")]
    pub id: i64,
    pub from_account_id: Uuid,
    pub date: chrono::DateTime<Utc>,
    #[sea_orm(column_type = "Text")]
    pub message: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
