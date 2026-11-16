use chrono::Utc;
use sea_orm::entity::prelude::*;

use crate::JsonGameSettings;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "games")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    pub date: chrono::DateTime<Utc>,

    pub is_rated: bool,
    pub game_settings: JsonGameSettings,

    pub player_white_id: Uuid,
    pub player_black_id: Uuid,
    pub player_white_username: String,
    pub player_black_username: String,
    pub player_white_rating: Option<f64>,
    pub player_black_rating: Option<f64>,

    pub rating_change_white: Option<f64>,
    pub rating_change_black: Option<f64>,
    pub result: Option<String>,

    #[sea_orm(indexed)]
    pub match_id: Option<i64>,
}

impl ActiveModelBehavior for ActiveModel {}
