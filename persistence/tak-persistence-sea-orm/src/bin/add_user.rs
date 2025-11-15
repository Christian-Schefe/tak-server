use sea_orm::{ActiveModelTrait, ColumnTrait, Database, EntityTrait, QueryFilter, Set};
use tak_persistence_sea_orm::entity::player;

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

    let players_db_url = format!("sqlite://{}?mode=rw", players_db_path);
    let db = Database::connect(&players_db_url)
        .await
        .expect("Failed to connect to database");

    create_user(&db, username, password, is_admin, is_mod).await;
}

async fn create_user(
    db: &sea_orm::DatabaseConnection,
    name: &str,
    password: &str,
    is_admin: bool,
    is_mod: bool,
) {
    let existing_user = player::Entity::find()
        .filter(player::Column::Name.eq(name))
        .one(db)
        .await
        .expect("Failed to query for existing user");

    if existing_user.is_some() {
        panic!("User with name [{}] already exists", name);
    }

    let pw_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST).expect("Failed to hash password");

    let new_user = player::ActiveModel {
        id: Default::default(), // Auto-increment
        name: Set(name.to_string()),
        password_hash: Set(pw_hash),
        email: Set(String::new()),
        rating: Set(1000.0),
        boost: Set(750.0),
        rated_games: Set(0),
        max_rating: Set(1000.0),
        rating_age: Set(0.0),
        unrated_games: Set(0),
        is_bot: Set(false),
        fatigue: Set("{}".to_string()),
        is_admin: Set(is_admin),
        is_mod: Set(is_mod || is_admin),
        is_gagged: Set(false),
        is_banned: Set(false),
        participation_rating: Set(1000),
    };

    new_user
        .insert(db)
        .await
        .expect("Failed to insert new user");

    println!("Created user [{}] with password [{}]", name, password);
}
