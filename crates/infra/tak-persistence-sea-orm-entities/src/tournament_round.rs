use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "tournament_rounds")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub tournament_id: i64,
    #[sea_orm(primary_key, auto_increment = false)]
    pub round_index: i64,
    pub data: serde_json::Value,
}

impl ActiveModelBehavior for ActiveModel {}
