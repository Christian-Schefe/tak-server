use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "profile")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: Uuid,
    pub display_name: Option<String>,
    pub country: Option<String>,
    pub profile_picture_version: Option<i64>,
}

impl ActiveModelBehavior for ActiveModel {}
