use chrono::Utc;
use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "ratings")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: Uuid,
    #[sea_orm(indexed)]
    pub rating: f64,
    pub boost: f64,
    pub rated_games: i32,
    pub max_rating: f64,
    pub rating_age: Option<chrono::DateTime<Utc>>,
    pub fatigue: serde_json::Value,
}

impl ActiveModelBehavior for ActiveModel {}
