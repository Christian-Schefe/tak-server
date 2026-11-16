use sea_orm::entity::prelude::*;

use crate::{JsonMatchSettings, JsonMatchStatus};

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "matches")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub match_id: i64,
    pub player1_id: Uuid,
    pub player2_id: Uuid,
    pub initial_color: String,
    pub settings: JsonMatchSettings,
    pub status: JsonMatchStatus,
    pub games_played: i32,
    pub score_player1: i32,
    pub score_player2: i32,
    #[sea_orm(indexed)]
    pub tournament_id: Option<i64>,
    pub tournament_round: Option<i64>,
    pub tournament_round_match_number: Option<i64>,
}

impl ActiveModelBehavior for ActiveModel {}
