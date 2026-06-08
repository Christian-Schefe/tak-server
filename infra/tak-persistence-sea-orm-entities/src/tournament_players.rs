use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "tournament_players")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, indexed)]
    pub tournament_id: i64,
    #[sea_orm(primary_key, auto_increment = false)]
    pub player_id: Uuid,
    pub half_score: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
