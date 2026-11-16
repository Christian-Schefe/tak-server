use sea_orm::entity::prelude::*;

use crate::JsonGameEvent;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "game_history")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    #[sea_orm(indexed)]
    pub game_id: i64,
    pub event: JsonGameEvent,
}

impl ActiveModelBehavior for ActiveModel {}
