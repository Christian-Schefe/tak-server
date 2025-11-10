use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "games")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    pub date: i64,
    pub size: i32,
    pub player_white: String,
    pub player_black: String,
    pub notation: String,
    pub result: String,
    pub timertime: i32,
    pub timerinc: i32,
    pub rating_white: i32,
    pub rating_black: i32,
    pub unrated: bool,
    pub tournament: bool,
    pub komi: i32,
    pub pieces: i32,
    pub capstones: i32,
    pub rating_change_white: i32,
    pub rating_change_black: i32,
    pub extra_time_amount: i32,
    pub extra_time_trigger: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
