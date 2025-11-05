pub mod games;
pub mod players;
use sqlx::{
    Pool, Sqlite,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};

fn create_player_db_pool() -> Pool<Sqlite> {
    let db_path = std::env::var("TAK_PLAYER_DB").expect("TAK_PLAYER_DB env var not set");

    let conn_options = SqliteConnectOptions::new()
        .filename(&db_path)
        .create_if_missing(false);

    SqlitePoolOptions::new()
        .max_connections(5)
        .connect_lazy_with(conn_options)
}

fn create_games_db_pool() -> Pool<Sqlite> {
    let db_path = std::env::var("TAK_GAMES_DB").expect("TAK_GAMES_DB env var not set");

    let conn_options = SqliteConnectOptions::new()
        .filename(&db_path)
        .create_if_missing(false);

    SqlitePoolOptions::new()
        .max_connections(5)
        .connect_lazy_with(conn_options)
}
