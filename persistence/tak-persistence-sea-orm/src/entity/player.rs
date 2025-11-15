use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "players")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    pub name: String,
    pub email: String,
    pub password_hash: String,
    pub rating: f64,
    pub boost: f64,
    pub rated_games: i32,
    pub unrated_games: i32,
    pub max_rating: f64,
    pub rating_age: f64,
    pub fatigue: String,
    pub is_bot: bool,
    pub is_gagged: bool,
    pub is_mod: bool,
    pub is_admin: bool,
    pub is_banned: bool,
    pub participation_rating: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
