use sqlx::{
    MySql, Pool,
    mysql::{MySqlConnectOptions, MySqlPoolOptions},
};

fn create_db_pool() -> Pool<MySql> {
    let db_username = std::env::var("MARIADB_USERNAME").expect("MARIADB_USERNAME env var not set");
    let db_password = std::env::var("MARIADB_PASSWORD").expect("MARIADB_PASSWORD env var not set");
    let db_host = std::env::var("MARIADB_HOST").expect("MARIADB_HOST env var not set");
    let db_port = std::env::var("MARIADB_PORT").expect("MARIADB_PORT env var not set");
    let db_database = std::env::var("MARIADB_DATABASE").expect("MARIADB_DATABASE env var not set");

    let conn_options = MySqlConnectOptions::new()
        .username(&db_username)
        .password(&db_password)
        .host(&db_host)
        .port(db_port.parse().expect("Invalid MARIADB_PORT"))
        .database(&db_database);

    MySqlPoolOptions::new()
        .max_connections(5)
        .connect_lazy_with(conn_options)
}
