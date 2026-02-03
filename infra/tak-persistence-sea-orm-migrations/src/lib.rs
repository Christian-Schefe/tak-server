use sea_orm::{DbErr, SchemaBuilder};
use sea_orm_migration::{MigrationTrait, MigratorTrait, SchemaManager};

mod m20260203_000001_create_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![]
        //vec![Box::new(m20260203_000001_create_table::Migration)]
    }
}

async fn sync_entities<'a>(manager: &SchemaManager<'a>) -> Result<(), DbErr> {
    let db = manager.get_connection();
    register_entities(db.get_schema_builder()).sync(db).await
}

fn register_entities(builder: SchemaBuilder) -> SchemaBuilder {
    builder
        .register(tak_persistence_sea_orm_entities::game::Entity)
        .register(tak_persistence_sea_orm_entities::player_account_mapping::Entity)
        .register(tak_persistence_sea_orm_entities::profile::Entity)
        .register(tak_persistence_sea_orm_entities::rating::Entity)
        .register(tak_persistence_sea_orm_entities::stats::Entity)
}
