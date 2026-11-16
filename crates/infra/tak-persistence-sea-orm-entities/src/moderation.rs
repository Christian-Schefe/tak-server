use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "moderation")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: Uuid,
    pub user_type: String,
    pub role: String,
    pub banned: bool,
    pub silenced: bool,
}

impl ActiveModelBehavior for ActiveModel {}
