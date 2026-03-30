use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "tournament_player_registration")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, indexed)]
    pub tournament_id: i64,
    #[sea_orm(primary_key, auto_increment = false)]
    pub player_id: Uuid,
    pub score: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
