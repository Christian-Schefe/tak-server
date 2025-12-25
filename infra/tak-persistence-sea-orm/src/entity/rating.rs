use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "players")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub player_id: Uuid,
    pub rating: f64,
    pub boost: f64,
    pub rated_games: i32,
    pub is_unrated: bool,
    pub max_rating: f64,
    pub rating_age: f64,
    pub participation_rating: f64,
    pub fatigue: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
