use std::{str::FromStr, sync::Arc};

use crate::create_db_pool;
use country_code_enum::CountryCode;
use sea_orm::{DatabaseConnection, EntityTrait, Iterable, sea_query::OnConflict};
use tak_domain::{
    UserId,
    profile::{AccountProfile, ProfilePictureVersion},
};
use tak_persistence_sea_orm_entities::profile;
use tak_server_app::ports::{RepoError, RepoRetrieveError, profile::AccountProfileRepository};

pub struct ProfileRepositoryImpl {
    db: DatabaseConnection,
    profile_cache: Arc<moka::sync::Cache<UserId, AccountProfile>>,
}

impl ProfileRepositoryImpl {
    pub async fn new() -> Self {
        let db = create_db_pool().await;
        let profile_cache = Arc::new(
            moka::sync::Cache::builder()
                .max_capacity(10_000)
                .time_to_live(std::time::Duration::from_secs(60 * 5))
                .build(),
        );
        Self { db, profile_cache }
    }
}

#[async_trait::async_trait]
impl AccountProfileRepository for ProfileRepositoryImpl {
    async fn insert_profile_information(
        &self,
        user_id: UserId,
        profile_information: AccountProfile,
    ) -> Result<(), RepoError> {
        let active_model = profile::ActiveModel {
            user_id: sea_orm::ActiveValue::Set(user_id.0),
            display_name: sea_orm::ActiveValue::Set(profile_information.display_name.clone()),
            country: sea_orm::ActiveValue::Set(profile_information.country.map(|x| x.to_string())),
            profile_picture_version: sea_orm::ActiveValue::Set(
                profile_information.profile_picture_version.map(|x| x.0),
            ),
        };
        profile::Entity::insert(active_model)
            .on_conflict(
                OnConflict::column(profile::Column::UserId)
                    .update_columns(
                        profile::Column::iter()
                            .filter(|c| !matches!(c, profile::Column::UserId))
                            .collect::<Vec<_>>(),
                    )
                    .to_owned(),
            )
            .exec(&self.db)
            .await
            .map_err(|e| RepoError::StorageError(e.to_string()))?;

        self.profile_cache
            .insert(user_id.clone(), profile_information);
        Ok(())
    }

    async fn get_profile_information(
        &self,
        user_id: UserId,
    ) -> Result<AccountProfile, RepoRetrieveError> {
        if let Some(cached_profile) = self.profile_cache.get(&user_id) {
            return Ok(cached_profile);
        }

        let profile_model = profile::Entity::find_by_id(user_id.0)
            .one(&self.db)
            .await
            .map_err(|e| RepoRetrieveError::StorageError(e.to_string()))?;

        if let Some(model) = profile_model {
            let profile_information = AccountProfile {
                display_name: model.display_name,
                country: model
                    .country
                    .as_deref()
                    .and_then(|c| CountryCode::from_str(c).ok()),
                profile_picture_version: model.profile_picture_version.map(ProfilePictureVersion),
            };
            self.profile_cache
                .insert(user_id.clone(), profile_information.clone());
            Ok(profile_information)
        } else {
            Err(RepoRetrieveError::NotFound)
        }
    }
}
