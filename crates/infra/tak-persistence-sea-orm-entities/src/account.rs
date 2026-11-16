use sea_orm::entity::prelude::*;

#[sea_orm::model]
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
    pub password_hash: Option<String>,
}

impl ActiveModelBehavior for ActiveModel {}
