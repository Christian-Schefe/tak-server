use sea_orm::{ConnectOptions, Database, DatabaseConnection};

pub mod entity;
pub mod games;
pub mod players;

async fn create_player_db_pool() -> DatabaseConnection {
    let db_path = std::env::var("TAK_PLAYER_DB").expect("TAK_PLAYER_DB env var not set");
    let db_url = format!("sqlite://{}?mode=rw", db_path);

    let mut opt = ConnectOptions::new(&db_url);
    opt.max_connections(5);

    Database::connect(opt)
        .await
        .expect("Failed to connect to player database")
}

async fn create_games_db_pool() -> DatabaseConnection {
    let db_path = std::env::var("TAK_GAMES_DB").expect("TAK_GAMES_DB env var not set");
    let db_url = format!("sqlite://{}?mode=rw", db_path);

    let mut opt = ConnectOptions::new(&db_url);
    opt.max_connections(5);

    Database::connect(opt)
        .await
        .expect("Failed to connect to games database")
}
