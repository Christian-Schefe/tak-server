use std::{str::FromStr, sync::Arc};

use crate::create_db_pool;
use country_code_enum::CountryCode;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait};
use tak_persistence_sea_orm_entites::profile;
use tak_server_app::domain::{
    AccountId, RepoError, RepoRetrieveError,
    profile::{AccountProfile, AccountProfileRepository},
};

pub struct ProfileRepositoryImpl {
    db: DatabaseConnection,
    profile_cache: Arc<moka::sync::Cache<AccountId, AccountProfile>>,
}

impl ProfileRepositoryImpl {
    pub async fn new() -> Self {
        let db = create_db_pool().await;
        let profile_cache = Arc::new(
            moka::sync::Cache::builder()
                .max_capacity(10_000)
                .time_to_live(std::time::Duration::from_secs(60 * 60 * 12))
                .build(),
        );
        Self { db, profile_cache }
    }
}

#[async_trait::async_trait]
impl AccountProfileRepository for ProfileRepositoryImpl {
    async fn insert_profile_information(
        &self,
        account_id: &AccountId,
        profile_information: AccountProfile,
    ) -> Result<(), RepoError> {
        let active_model = profile::ActiveModel {
            account_id: sea_orm::ActiveValue::Set(account_id.to_string()),
            country: sea_orm::ActiveValue::Set(profile_information.country.map(|x| x.to_string())),
        };
        active_model
            .insert(&self.db)
            .await
            .map_err(|e| RepoError::StorageError(e.to_string()))?;
        self.profile_cache
            .insert(account_id.clone(), profile_information);
        Ok(())
    }

    async fn get_profile_information(
        &self,
        account_id: &AccountId,
    ) -> Result<AccountProfile, RepoRetrieveError> {
        if let Some(cached_profile) = self.profile_cache.get(account_id) {
            return Ok(cached_profile);
        }

        let profile_model = profile::Entity::find_by_id(account_id.to_string())
            .one(&self.db)
            .await
            .map_err(|e| RepoRetrieveError::StorageError(e.to_string()))?;

        if let Some(model) = profile_model {
            let profile_information = AccountProfile {
                country: model
                    .country
                    .as_deref()
                    .and_then(|c| CountryCode::from_str(c).ok()),
            };
            self.profile_cache
                .insert(account_id.clone(), profile_information.clone());
            Ok(profile_information)
        } else {
            Err(RepoRetrieveError::NotFound)
        }
    }
}
