use sqlx::{
    Pool, Sqlite,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};

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

    let players_connect_options = SqliteConnectOptions::new()
        .filename(&players_db_path)
        .create_if_missing(true);

    let players_conn = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(players_connect_options)
        .await
        .expect("Failed to create pool");

    create_user(&players_conn, username, password).await;
}

async fn create_user(conn: &Pool<Sqlite>, name: &str, password: &str) {
    let sql = "INSERT INTO players (name, password, email, rating, boost, ratedgames, maxrating, ratingage, ratingbase, unrated, isbot, fatigue, is_admin, is_mod, is_gagged, is_banned) VALUES(?, ?,'',1000.0,750.0,0,1000.0,0,0,0,0,'{}',0,false,false,0);";
    let pw_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST).expect("Failed to hash password");

    sqlx::query(sql)
        .bind(name)
        .bind(pw_hash)
        .execute(conn)
        .await
        .expect("Failed to insert new user");

    println!("Created user [{}] with password [{}]", name, password);
}
