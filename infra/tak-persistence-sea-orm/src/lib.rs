use async_lock::OnceCell;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};

pub mod games;
pub mod player_account_mapping;
pub mod profile;
pub mod ratings;
pub mod stats;

static DB_POOL: OnceCell<DatabaseConnection> = OnceCell::new();

async fn try_reconnect_db_pool(opt: ConnectOptions) -> DatabaseConnection {
    loop {
        match Database::connect(opt.clone()).await {
            Ok(db) => return db,
            Err(e) => {
                eprintln!(
                    "Failed to connect to database: {}. Retrying in 5 seconds...",
                    e
                );
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    }
}

pub async fn create_db_pool() -> DatabaseConnection {
    DB_POOL
        .get_or_init(|| async move {
            let mariadb_database =
                std::env::var("MARIADB_DATABASE").expect("MARIADB_DATABASE must be set");
            let mariadb_user = std::env::var("MARIADB_USER").expect("MARIADB_USER must be set");
            let mariadb_password =
                std::env::var("MARIADB_PASSWORD").expect("MARIADB_PASSWORD must be set");
            let mariadb_host = std::env::var("MARIADB_HOST").expect("MARIADB_HOST must be set");
            let mariadb_port = std::env::var("MARIADB_PORT").expect("MARIADB_PORT must be set");
            let db_url = format!(
                "mysql://{}:{}@{}:{}/{}",
                mariadb_user, mariadb_password, mariadb_host, mariadb_port, mariadb_database
            );

            log::info!("Connecting to database at {}", db_url);

            let mut opt = ConnectOptions::new(&db_url);
            opt.max_connections(5);

            let db = try_reconnect_db_pool(opt).await;

            db.get_schema_builder()
                .register(tak_persistence_sea_orm_entites::game::Entity)
                .register(tak_persistence_sea_orm_entites::player_account_mapping::Entity)
                .register(tak_persistence_sea_orm_entites::profile::Entity)
                .register(tak_persistence_sea_orm_entites::rating::Entity)
                .register(tak_persistence_sea_orm_entites::stats::Entity)
                .sync(&db)
                .await
                .expect("Failed to apply entity sync");
            db
        })
        .await
        .clone()
}
