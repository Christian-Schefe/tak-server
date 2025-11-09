use sqlx::{
    Pool, Sqlite,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 && args.len() != 4 {
        eprintln!("Usage: add_user <username> <password> [<role>]");
        std::process::exit(1);
    }

    let players_db_path = std::env::var("TAK_PLAYER_DB").expect("TAK_PLAYER_DB env var not set");

    let username = &args[1];
    let password = &args[2];
    let role = if args.len() == 4 { &args[3] } else { "" };

    let is_admin = role == "admin";
    let is_mod = role == "mod";

    let players_connect_options = SqliteConnectOptions::new()
        .filename(&players_db_path)
        .create_if_missing(true);

    let players_conn = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(players_connect_options)
        .await
        .expect("Failed to create pool");

    create_user(&players_conn, username, password, is_admin, is_mod).await;
}

async fn create_user(
    conn: &Pool<Sqlite>,
    name: &str,
    password: &str,
    is_admin: bool,
    is_mod: bool,
) {
    let largest_id = match sqlx::query_scalar::<_, i64>("SELECT MAX(id) FROM players")
        .fetch_one(conn)
        .await
    {
        Ok(id) => id,
        Err(sqlx::Error::RowNotFound) => 0,
        Err(e) => panic!("Failed to get largest player id: {}", e),
    };

    let existing_user: Option<(i64,)> = sqlx::query_as("SELECT id FROM players WHERE name = ?")
        .bind(name)
        .fetch_optional(conn)
        .await
        .expect("Failed to query for existing user");

    if existing_user.is_some() {
        panic!("User with name [{}] already exists", name);
    }

    let sql = "INSERT INTO players (id, name, password, email, rating, boost, ratedgames, maxrating, ratingage, ratingbase, unrated, isbot, fatigue, is_admin, is_mod, is_gagged, is_banned) VALUES(?, ?, ?,'',1000.0,750.0,0,1000.0,0,0,0,0,'{}', ?, ?, false, 0);";
    let pw_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST).expect("Failed to hash password");

    sqlx::query(sql)
        .bind(largest_id + 1)
        .bind(name)
        .bind(pw_hash)
        .bind(is_admin)
        .bind(is_mod || is_admin)
        .execute(conn)
        .await
        .expect("Failed to insert new user");

    println!("Created user [{}] with password [{}]", name, password);
}
