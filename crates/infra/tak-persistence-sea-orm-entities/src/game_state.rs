use sea_orm::entity::prelude::*;

use crate::{JsonClock, JsonGameRequests, JsonHistory};

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "game_states")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i64,
    pub history: JsonHistory,
    pub clock: JsonClock,
    pub white_requests: JsonGameRequests,
    pub black_requests: JsonGameRequests,
}

impl ActiveModelBehavior for ActiveModel {}
