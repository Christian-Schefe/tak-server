use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "accounts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: Uuid,
    pub username: String,
    #[sea_orm(unique)]
    pub unique_name: Option<String>,
    #[sea_orm(unique)]
    pub email: Option<String>,
    pub account_type: String,
    pub role: String,
    pub banned: bool,
    pub silenced: bool,
    pub password_hash: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
