use sqlx::{Pool, Sqlite};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: add_user <username> <password>");
        std::process::exit(1);
    }

    let players_db_path = std::env::var("TAK_PLAYER_DB").expect("TAK_PLAYER_DB env var not set");

    let username = &args[1];
    let password = &args[2];
    let conn = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&players_db_path)
        .await
        .expect("Failed to create pool");
    create_user(&conn, username, password).await;
}

async fn create_user(conn: &Pool<Sqlite>, name: &str, password: &str) {
    let next_id: i64 = sqlx::query_scalar("SELECT IFNULL(MAX(id), 0) + 1 FROM players;")
        .fetch_one(conn)
        .await
        .expect("Failed to get next user ID");
    let sql = "INSERT INTO players (id, name, password, email, rating, boost, ratedgames, maxrating, ratingage, ratingbase, unrated, isbot, fatigue, is_admin, is_mod, is_gagged, is_banned) VALUES(?, ?, ?,'',1000.0,750.0,0,1000.0,0,0,0,0,'{}',0,false,false,0);";
    let pw_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST).expect("Failed to hash password");

    sqlx::query(sql)
        .bind(next_id)
        .bind(name)
        .bind(pw_hash)
        .execute(conn)
        .await
        .expect("Failed to insert new user");

    println!("Created user [{}] with password [{}]", name, password);
}
