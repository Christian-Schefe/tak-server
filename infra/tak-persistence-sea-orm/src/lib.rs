use async_lock::OnceCell;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};

pub mod entity;
pub mod games;
pub mod player_account_mapping;
pub mod profile;
pub mod ratings;

static DB_POOL: OnceCell<DatabaseConnection> = OnceCell::new();

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

            let mut opt = ConnectOptions::new(&db_url);
            opt.max_connections(5);

            Database::connect(opt)
                .await
                .expect("Failed to connect to database")
        })
        .await
        .clone()
}
