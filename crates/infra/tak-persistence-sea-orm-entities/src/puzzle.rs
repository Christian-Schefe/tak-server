use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "puzzles")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i64,
    pub size: i32,
    pub half_komi: i32,
    pub pieces: i32,
    pub capstones: i32,
    pub opening: String,
    pub position: serde_json::Value,
    pub responses: serde_json::Value,
    pub random_seed: f64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
