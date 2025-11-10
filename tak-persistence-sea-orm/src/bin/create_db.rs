use sea_orm::{ConnectionTrait, Database, DatabaseBackend, Schema};
use tak_persistence_sea_orm::entity::{game, player};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let games_db_path = std::env::var("TAK_GAMES_DB").expect("TAK_GAMES_DB env var not set");
    let players_db_path = std::env::var("TAK_PLAYER_DB").expect("TAK_PLAYER_DB env var not set");

    if std::path::Path::new(&games_db_path).exists() {
        std::fs::remove_file(&games_db_path).expect("Failed to remove existing games DB");
        println!("Removed existing games DB at {}", games_db_path);
    }
    if std::path::Path::new(&players_db_path).exists() {
        std::fs::remove_file(&players_db_path).expect("Failed to remove existing players DB");
        println!("Removed existing players DB at {}", players_db_path);
    }

    // Create games database
    let games_db_url = format!("sqlite://{}?mode=rwc", games_db_path);
    let games_db = Database::connect(&games_db_url)
        .await
        .expect("Failed to connect to games database");

    let schema = Schema::new(DatabaseBackend::Sqlite);
    let stmt = schema.create_table_from_entity(game::Entity);

    games_db
        .execute(&stmt)
        .await
        .expect("Failed to create games table");

    println!("Created new games DB at {}", games_db_path);

    // Create players database
    let players_db_url = format!("sqlite://{}?mode=rwc", players_db_path);
    let players_db = Database::connect(&players_db_url)
        .await
        .expect("Failed to connect to players database");

    let stmt = schema.create_table_from_entity(player::Entity);

    players_db
        .execute(&stmt)
        .await
        .expect("Failed to create players table");

    println!("Created new players DB at {}", players_db_path);
}
