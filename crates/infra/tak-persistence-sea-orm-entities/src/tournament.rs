use sea_orm::entity::prelude::*;

use crate::{JsonMatchSettings, JsonTournamentFormat, JsonTournamentStatus};

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "tournaments")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    pub name: String,
    pub format: JsonTournamentFormat,
    pub status: JsonTournamentStatus,
    pub match_settings: JsonMatchSettings,
}

impl ActiveModelBehavior for ActiveModel {}
