use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

fn main() {
    dotenvy::dotenv().ok();

    let players_db_sql = "CREATE TABLE players (id INT PRIMARY_KEY, name VARCHAR(20), password VARCHAR(50), email VARCHAR(50), rating real default 1000, boost real default 750, ratedgames int default 0, maxrating real default 1000, ratingage real default 0, ratingbase int default 0, unrated int default 0, isbot int default 0, fatigue text default '{}', is_admin int default 0, is_mod int default 0, is_gagged int default 0, is_banned int default 0, participation_rating int default 1000);";
    let games_db_sql = "CREATE TABLE games (id INTEGER PRIMARY KEY, date INT, size INT, player_white VARCHAR(20), player_black VARCHAR(20), notation TEXT, result VARCAR(10), timertime INT DEFAULT 0, timerinc INT DEFAULT 0, rating_white int default 1000, rating_black int default 1000, unrated int default 0, tournament int default 0, komi int default 0, pieces int default -1, capstones int default -1, rating_change_white int default 0, rating_change_black int default 0, extra_time_amount int default 0, extra_time_trigger int default 0);";

    let games_db_path = std::env::var("TAK_GAMES_DB").expect("TAK_GAMES_DB env var not set");
    let players_db_path = std::env::var("TAK_PLAYER_DB").expect("TAK_PLAYER_DB env var not set");
    let parent = std::path::Path::new(&games_db_path)
        .parent()
        .expect("Failed to get parent directory of games DB path");
    if !parent.exists() {
        std::fs::create_dir_all(parent).expect("Failed to create parent directory for games DB");
        println!(
            "Created parent directory for games DB at {}",
            parent.display()
        );
    }

    if std::path::Path::new(&games_db_path).exists() {
        std::fs::remove_file(&games_db_path).expect("Failed to remove existing games DB");
        println!("Removed existing games DB at {}", games_db_path);
    }
    if std::path::Path::new(&players_db_path).exists() {
        std::fs::remove_file(&players_db_path).expect("Failed to remove existing players DB");
        println!("Removed existing players DB at {}", players_db_path);
    }

    let games_db_manager = SqliteConnectionManager::file(&games_db_path);
    let games_db_pool = Pool::builder()
        .max_size(5)
        .build(games_db_manager)
        .expect("Failed to create DB pool");
    let conn = games_db_pool.get().expect("Failed to get DB connection");
    conn.execute_batch(games_db_sql)
        .expect("Failed to create games table");

    println!("Created new games DB at {}", games_db_path);

    let players_db_manager = SqliteConnectionManager::file(&players_db_path);
    let players_db_pool = Pool::builder()
        .max_size(5)
        .build(players_db_manager)
        .expect("Failed to create DB pool");
    let conn = players_db_pool.get().expect("Failed to get DB connection");
    conn.execute_batch(players_db_sql)
        .expect("Failed to create players table");

    println!("Created new players DB at {}", players_db_path);

    create_user(&conn, "testuser", "pw");
    create_user(&conn, "testuser2", "pw");
}

fn create_user(conn: &rusqlite::Connection, name: &str, password: &str) {
    let next_id: i64 = conn
        .query_row("SELECT IFNULL(MAX(id), 0) + 1 FROM players", [], |row| {
            row.get(0)
        })
        .expect("Failed to get next user ID");
    let sql = "INSERT INTO players (id, name, password, email, rating, boost, ratedgames, maxrating, ratingage, ratingbase, unrated, isbot, fatigue, is_admin, is_mod, is_gagged, is_banned) VALUES(?1, ?2, ?3,'',1000.0,750.0,0,1000.0,0,0,0,0,'{}',0,0,0,0);";
    let pw_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST).expect("Failed to hash password");
    conn.execute(sql, [next_id.to_string(), name.to_string(), pw_hash])
        .expect("Failed to create user");
    println!("Created user {}", name);
}
