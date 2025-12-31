use chrono::Utc;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "games")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    pub date: chrono::DateTime<Utc>,
    pub size: i32,

    pub player_white_id: Uuid,
    pub player_black_id: Uuid,
    pub player_white_username: Option<String>,
    pub player_black_username: Option<String>,
    pub player_white_rating: Option<f64>,
    pub player_black_rating: Option<f64>,

    pub rating_change_white: Option<f64>,
    pub rating_change_black: Option<f64>,

    pub notation: String,
    pub result: String,
    pub clock_contingent: i32,
    pub clock_increment: i32,
    pub is_unrated: bool,
    pub is_tournament: bool,
    pub half_komi: i32,
    pub pieces: i32,
    pub capstones: i32,
    pub extra_time_amount: i32,
    pub extra_time_trigger: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
