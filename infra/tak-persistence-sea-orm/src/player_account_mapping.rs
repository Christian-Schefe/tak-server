use std::sync::Arc;

use crate::create_db_pool;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use tak_persistence_sea_orm_entites::player_account_mapping;
use tak_server_app::{
    domain::{AccountId, PlayerId, RepoError, RepoRetrieveError},
    ports::player_mapping::PlayerAccountMappingRepository,
};

pub struct PlayerAccountMappingRepositoryImpl {
    db: DatabaseConnection,
    player_id_to_account_id_cache: Arc<moka::sync::Cache<PlayerId, AccountId>>,
    account_id_to_player_id_cache: Arc<moka::sync::Cache<AccountId, PlayerId>>,
}

impl PlayerAccountMappingRepositoryImpl {
    pub async fn new() -> Self {
        let db = create_db_pool().await;
        let player_id_to_account_id_cache = Arc::new(
            moka::sync::Cache::builder()
                .max_capacity(10_000)
                .time_to_live(std::time::Duration::from_secs(60 * 60))
                .build(),
        );
        let account_id_to_player_id_cache = Arc::new(
            moka::sync::Cache::builder()
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

    async fn get_by_account_id(
        &self,
        account_id: &AccountId,
    ) -> Result<Option<PlayerId>, RepoError> {
        let player_model = player_account_mapping::Entity::find_by_id(account_id.to_string())
            .one(&self.db)
            .await
            .map_err(|e| RepoError::StorageError(e.to_string()))?;
        if let Some(model) = &player_model {
            let player_id = PlayerId(model.player_id);
            self.account_id_to_player_id_cache
                .insert(account_id.clone(), player_id.clone());
            self.player_id_to_account_id_cache
                .insert(player_id.clone(), account_id.clone());
            Ok(Some(player_id))
        } else {
            Ok(None)
        }
    }
}

#[async_trait::async_trait]
impl PlayerAccountMappingRepository for PlayerAccountMappingRepositoryImpl {
    async fn get_account_id(&self, player_id: PlayerId) -> Result<AccountId, RepoRetrieveError> {
        if let Some(account_id) = self.player_id_to_account_id_cache.get(&player_id) {
            return Ok(account_id);
        }

        let player_model = player_account_mapping::Entity::find()
            .filter(player_account_mapping::Column::PlayerId.eq(player_id.0))
            .one(&self.db)
            .await
            .map_err(|e| RepoRetrieveError::StorageError(e.to_string()))?;

        if let Some(model) = player_model {
            let account_id = AccountId(model.account_id);

            self.account_id_to_player_id_cache
                .insert(account_id.clone(), player_id);
            self.player_id_to_account_id_cache
                .insert(player_id, account_id.clone());

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
        if let Some(player_id) = self.account_id_to_player_id_cache.get(account_id) {
            return Ok(player_id);
        }

        if let Some(player_id) = self.get_by_account_id(account_id).await? {
            return Ok(player_id);
        }

        let new_player_id = create_fn();
        let active_model = player_account_mapping::ActiveModel {
            player_id: Set(new_player_id.0),
            account_id: Set(account_id.to_string()),
        };

        let res = active_model
            .insert(&self.db)
            .await
            .map_err(|e| RepoError::StorageError(e.to_string()))?;

        let player_id = PlayerId(res.player_id);

        self.account_id_to_player_id_cache
            .insert(account_id.clone(), player_id.clone());
        self.player_id_to_account_id_cache
            .insert(player_id.clone(), account_id.clone());
        Ok(player_id)
    }

    async fn remove_account_id(
        &self,
        account_id: &AccountId,
    ) -> Result<Option<PlayerId>, RepoError> {
        // This is in theory a race condition, but an assigned player id never changes, so nothing bad
        // can happen.
        let Some(player_id) = self.get_by_account_id(account_id).await? else {
            return Ok(None);
        };
        let res = player_account_mapping::Entity::delete_by_id(account_id.to_string())
            .exec(&self.db)
            .await
            .map_err(|e| RepoError::StorageError(e.to_string()))?;

        self.account_id_to_player_id_cache.invalidate(account_id);
        self.player_id_to_account_id_cache.invalidate(&player_id);

        if res.rows_affected == 0 {
            log::warn!(
                "Tried to remove player mapping for account_id {:?}, but no rows were affected",
                account_id
            );
            return Ok(None);
        }

        Ok(Some(player_id))
    }
}
