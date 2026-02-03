use sea_orm_migration::prelude::*;

use crate::sync_entities;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        sync_entities(manager).await?;
        Ok(())
    }
}
