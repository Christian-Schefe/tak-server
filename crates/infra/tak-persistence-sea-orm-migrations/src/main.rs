use sea_orm_migration::prelude::*;

#[tokio::main]
async fn main() {
    cli::run_cli(tak_persistence_sea_orm_migrations::Migrator).await;
}
