use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "matches")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub match_id: i64,
    pub player1_id: Uuid,
    pub player2_id: Uuid,
    pub initial_color: String,
    pub game_settings: serde_json::Value,
    pub status: String,
    pub match_mode: serde_json::Value,
    pub games_played: u32,
    pub half_score_player1: u32,
    pub half_score_player2: u32,
    pub is_rated: bool,
    #[sea_orm(indexed)]
    pub tournament_id: Option<i64>,
    pub tournament_round: Option<i64>,
    pub tournament_round_match_number: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
