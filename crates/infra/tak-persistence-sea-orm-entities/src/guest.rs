use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "guests")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub guest_number: i64,
    #[sea_orm(unique)]
    pub account_id: Uuid,
    pub banned: bool,
    pub silenced: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
