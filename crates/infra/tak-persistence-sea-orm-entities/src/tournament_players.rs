use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "tournament_players")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, indexed)]
    pub tournament_id: i64,
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: Uuid,
    pub score: i32,
    pub seeding_score: i32,
}

impl ActiveModelBehavior for ActiveModel {}
