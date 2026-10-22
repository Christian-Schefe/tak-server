use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "game_states")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub history: serde_json::Value,
    pub white_remaining_ms: i64,
    pub black_remaining_ms: i64,
    pub last_clock_update: chrono::DateTime<chrono::Utc>,
    pub is_clock_ticking: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
