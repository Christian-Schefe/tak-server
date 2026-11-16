use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "stats")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: Uuid,
    pub rated_games_played: i32,
    pub games_played: i32,
    pub games_won: i32,
    pub games_lost: i32,
    pub games_drawn: i32,
    pub win_streak: i32,
    pub longest_win_streak: i32,
}

impl ActiveModelBehavior for ActiveModel {}
