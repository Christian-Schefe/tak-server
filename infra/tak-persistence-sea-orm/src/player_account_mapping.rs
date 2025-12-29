use std::sync::Arc;

use crate::{create_db_pool, entity::player_account_mapping};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
    TransactionError, TransactionTrait,
};
use tak_server_app::{
    domain::{AccountId, PlayerId, RepoError, RepoRetrieveError},
    ports::player_mapping::PlayerAccountMappingRepository,
};

pub struct PlayerAccountMappingRepositoryImpl {
    db: DatabaseConnection,
    player_id_to_account_id_cache: Arc<moka::future::Cache<PlayerId, AccountId>>,
    account_id_to_player_id_cache: Arc<moka::future::Cache<AccountId, PlayerId>>,
}

impl PlayerAccountMappingRepositoryImpl {
    pub async fn new() -> Self {
        let db = create_db_pool().await;
        let player_id_to_account_id_cache = Arc::new(
            moka::future::Cache::builder()
                .max_capacity(10_000)
                .time_to_live(std::time::Duration::from_secs(60 * 60))
                .build(),
        );
        let account_id_to_player_id_cache = Arc::new(
            moka::future::Cache::builder()
                .max_capacity(10_000)
                .time_to_live(std::time::Duration::from_secs(60 * 60))
                .build(),
        );
        Self {
            db,
            player_id_to_account_id_cache,
            account_id_to_player_id_cache,
        }
    }
}

#[async_trait::async_trait]
impl PlayerAccountMappingRepository for PlayerAccountMappingRepositoryImpl {
    async fn get_account_id(&self, player_id: PlayerId) -> Result<AccountId, RepoRetrieveError> {
        if let Some(account_id) = self.player_id_to_account_id_cache.get(&player_id).await {
            return Ok(account_id);
        }

        let player_model = player_account_mapping::Entity::find_by_id(player_id.0)
            .one(&self.db)
            .await
            .map_err(|e| RepoRetrieveError::StorageError(e.to_string()))?;

        if let Some(model) = player_model {
            let account_id = AccountId(model.account_id);
            futures::join!(
                self.player_id_to_account_id_cache
                    .insert(player_id, account_id.clone()),
                self.account_id_to_player_id_cache
                    .insert(account_id.clone(), player_id)
            );
            Ok(account_id)
        } else {
            Err(RepoRetrieveError::NotFound)
        }
    }

    async fn get_or_create_player_id(
        &self,
        account_id: &AccountId,
        create_fn: impl FnOnce() -> PlayerId + Send + 'static,
    ) -> Result<PlayerId, RepoError> {
        if let Some(player_id) = self.account_id_to_player_id_cache.get(account_id).await {
            return Ok(player_id);
        }

        let acc_id_str = account_id.0.clone();
        let res = self
            .db
            .transaction::<_, PlayerId, RepoError>(|c| {
                Box::pin(async move {
                    let player_model = player_account_mapping::Entity::find()
                        .filter(player_account_mapping::Column::AccountId.eq(acc_id_str.clone()))
                        .one(c)
                        .await
                        .map_err(|e| RepoError::StorageError(e.to_string()))?;

                    if let Some(model) = player_model {
                        let player_id = PlayerId(model.player_id);
                        Ok(player_id)
                    } else {
                        let new_player_id = create_fn();
                        let active_model = player_account_mapping::ActiveModel {
                            player_id: Set(new_player_id.0),
                            account_id: Set(acc_id_str),
                        };
                        active_model
                            .insert(c)
                            .await
                            .map_err(|e| RepoError::StorageError(e.to_string()))?;
                        Ok(new_player_id)
                    }
                })
            })
            .await;
        match res {
            Ok(player_id) => {
                futures::join!(
                    self.account_id_to_player_id_cache
                        .insert(account_id.clone(), player_id),
                    self.player_id_to_account_id_cache
                        .insert(player_id, account_id.clone())
                );
                Ok(player_id)
            }
            Err(TransactionError::Transaction(e)) => Err(e),
            Err(TransactionError::Connection(e)) => Err(RepoError::StorageError(e.to_string())),
        }
    }
}
