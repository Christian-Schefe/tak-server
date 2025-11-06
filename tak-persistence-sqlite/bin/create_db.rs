use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let players_db_sql = "CREATE TABLE players (id INT PRIMARY_KEY, name VARCHAR(20), password VARCHAR(50), email VARCHAR(50), rating real default 1000, boost real default 750, ratedgames int default 0, maxrating real default 1000, ratingage real default 0, ratingbase int default 0, unrated int default 0, isbot int default 0, fatigue text default '{}', is_admin int default 0, is_mod int default 0, is_gagged int default 0, is_banned int default 0, participation_rating int default 1000);";
    let games_db_sql = "CREATE TABLE games (id INTEGER PRIMARY KEY, date INT, size INT, player_white VARCHAR(20), player_black VARCHAR(20), notation TEXT, result VARCAR(10), timertime INT DEFAULT 0, timerinc INT DEFAULT 0, rating_white int default 1000, rating_black int default 1000, unrated int default 0, tournament int default 0, komi int default 0, pieces int default -1, capstones int default -1, rating_change_white int default 0, rating_change_black int default 0, extra_time_amount int default 0, extra_time_trigger int default 0);";

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

    let games_connect_options = SqliteConnectOptions::new()
        .filename(&games_db_path)
        .create_if_missing(true);

    let games_conn = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(games_connect_options)
        .await
        .expect("Failed to create pool");

    sqlx::query(games_db_sql)
        .execute(&games_conn)
        .await
        .expect("Failed to create games table");

    println!("Created new games DB at {}", games_db_path);

    let players_connect_options = SqliteConnectOptions::new()
        .filename(&players_db_path)
        .create_if_missing(true);

    let players_conn = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(players_connect_options)
        .await
        .expect("Failed to create pool");

    sqlx::query(players_db_sql)
        .execute(&players_conn)
        .await
        .expect("Failed to create players table");

    println!("Created new players DB at {}", players_db_path);
}
