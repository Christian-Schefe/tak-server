use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "players")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i64,
    pub name: String,
    pub email: String,
    pub password: String,
    pub rating: f64,
    pub boost: f64,
    pub ratedgames: i32,
    pub maxrating: f64,
    pub ratingage: f64,
    pub ratingbase: i32,
    pub unrated: i32,
    pub isbot: bool,
    pub fatigue: String,
    pub is_gagged: bool,
    pub is_mod: bool,
    pub is_admin: bool,
    pub is_banned: bool,
    pub participation_rating: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
