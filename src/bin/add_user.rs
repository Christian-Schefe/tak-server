fn main() {
    dotenvy::dotenv().ok();

    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: add_user <username> <password>");
        std::process::exit(1);
    }

    let players_db_path = std::env::var("TAK_PLAYER_DB").expect("TAK_PLAYER_DB env var not set");
    let parent = std::path::Path::new(&players_db_path)
        .parent()
        .expect("Failed to get parent directory of players DB path");
    if !parent.exists() {
        std::fs::create_dir_all(parent).expect("Failed to create parent directory for players DB");
        println!(
            "Created parent directory for players DB at {}",
            parent.display()
        );
    }

    let username = &args[1];
    let password = &args[2];
    let conn = rusqlite::Connection::open(players_db_path).expect("Failed to open database");
    create_user(&conn, username, password);
}

fn create_user(conn: &rusqlite::Connection, name: &str, password: &str) {
    let next_id: i64 = conn
        .query_row("SELECT IFNULL(MAX(id), 0) + 1 FROM players", [], |row| {
            row.get(0)
        })
        .expect("Failed to get next user ID");
    let sql = "INSERT INTO players (id, name, password, email, rating, boost, ratedgames, maxrating, ratingage, ratingbase, unrated, isbot, fatigue, is_admin, is_mod, is_gagged, is_banned) VALUES(?1, ?2, ?3,'',1000.0,750.0,0,1000.0,0,0,0,0,'{}',0,false,false,0);";
    let pw_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST).expect("Failed to hash password");
    conn.execute(sql, [next_id.to_string(), name.to_string(), pw_hash])
        .expect("Failed to create user");
    println!("Created user [{}] with password [{}]", name, password);
}
