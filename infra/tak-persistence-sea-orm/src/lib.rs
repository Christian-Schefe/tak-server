use async_lock::OnceCell;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};

pub mod entity;
pub mod games;
pub mod players;
pub mod ratings;

static DB_POOL: OnceCell<DatabaseConnection> = OnceCell::new();

async fn create_db_pool() -> DatabaseConnection {
    DB_POOL
        .get_or_init(|| async move {
            let db_path = std::env::var("TAK_DB").expect("TAK_DB env var not set");
            let db_url = format!("sqlite://{}?mode=rw", db_path);

            let mut opt = ConnectOptions::new(&db_url);
            opt.max_connections(5);

            Database::connect(opt)
                .await
                .expect("Failed to connect to database")
        })
        .await
        .clone()
}
