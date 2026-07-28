use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "tournaments")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub tournament_id: i64,
    pub name: String,
    pub tournament_format_settings: serde_json::Value,
    pub status: String,
    pub match_settings: serde_json::Value,
    pub registration_open: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
