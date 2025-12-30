use sea_orm::{ConnectionTrait, DatabaseBackend, Schema};
use tak_persistence_sea_orm::{
    create_db_pool,
    entity::{game, player_account_mapping, profile, rating},
};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let pool = create_db_pool().await;

    let schema = Schema::new(DatabaseBackend::MySql);
    let game_table = schema.create_table_from_entity(game::Entity);
    let player_account_mapping_table =
        schema.create_table_from_entity(player_account_mapping::Entity);
    let profile_table = schema.create_table_from_entity(profile::Entity);
    let rating_table = schema.create_table_from_entity(rating::Entity);

    pool.execute(&game_table)
        .await
        .expect("Failed to create games table");

    pool.execute(&player_account_mapping_table)
        .await
        .expect("Failed to create player account mapping table");
    pool.execute(&profile_table)
        .await
        .expect("Failed to create profiles table");

    pool.execute(&rating_table)
        .await
        .expect("Failed to create ratings table");

    println!("Created database tables successfully");
}
