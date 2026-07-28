use sea_orm::{
    ActiveValue::{NotSet, Set},
    ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};
use tak_auth_ory::guest::{GuestAccount, GuestRepository};
use tak_persistence_sea_orm_entities::guest;
use tak_server_app::domain::{
    AccountId, RepoError, RepoRetrieveError, moderation::ModerationFlags,
};

use crate::{create_db_pool, db_error_to_repo_retrieve_error};

pub struct GuestRepositoryImpl {
    db: DatabaseConnection,
}

impl GuestRepositoryImpl {
    pub async fn new() -> Self {
        let db = create_db_pool().await;

        Self { db }
    }
}

#[async_trait::async_trait]
impl GuestRepository for GuestRepositoryImpl {
    async fn get_account(&self, account_id: &AccountId) -> Result<GuestAccount, RepoRetrieveError> {
        let model = guest::Entity::find()
            .filter(guest::Column::AccountId.eq(account_id.0))
            .one(&self.db)
            .await
            .map_err(|e| RepoRetrieveError::StorageError(e.to_string()))?;
        if let Some(model) = model {
            Ok(GuestAccount {
                account_id: AccountId(model.account_id),
                guest_number: model.guest_number,
                flags: ModerationFlags::new(model.banned, model.silenced),
            })
        } else {
            Err(RepoRetrieveError::NotFound)
        }
    }

    async fn get_by_guest_number(
        &self,
        guest_number: i64,
    ) -> Result<GuestAccount, RepoRetrieveError> {
        let model = guest::Entity::find()
            .filter(guest::Column::GuestNumber.eq(guest_number as i64))
            .one(&self.db)
            .await
            .map_err(|e| RepoRetrieveError::StorageError(e.to_string()))?;
        if let Some(model) = model {
            Ok(GuestAccount {
                account_id: AccountId(model.account_id),
                guest_number: model.guest_number,
                flags: ModerationFlags::new(model.banned, model.silenced),
            })
        } else {
            Err(RepoRetrieveError::NotFound)
        }
    }

    async fn create_guest(&self) -> Result<GuestAccount, RepoError> {
        let account_id = uuid::Uuid::new_v4();
        let flags = ModerationFlags::empty();
        let new_guest = guest::ActiveModel {
            account_id: Set(account_id),
            guest_number: NotSet,
            banned: Set(
                flags.is_flagged(tak_server_app::domain::moderation::ModerationFlag::Banned)
            ),
            silenced: Set(
                flags.is_flagged(tak_server_app::domain::moderation::ModerationFlag::Silenced)
            ),
        };
        let res = guest::Entity::insert(new_guest)
            .exec(&self.db)
            .await
            .map_err(|e| RepoError::StorageError(e.to_string()))?;
        Ok(GuestAccount {
            account_id: AccountId(account_id),
            guest_number: res.last_insert_id,
            flags,
        })
    }

    async fn update_guest(&self, account: &GuestAccount) -> Result<(), RepoRetrieveError> {
        let model = guest::ActiveModel {
            account_id: NotSet,
            guest_number: Set(account.guest_number as i64),
            banned: Set(account
                .flags
                .is_flagged(tak_server_app::domain::moderation::ModerationFlag::Banned)),
            silenced: Set(account
                .flags
                .is_flagged(tak_server_app::domain::moderation::ModerationFlag::Silenced)),
        };
        guest::Entity::update(model)
            .exec(&self.db)
            .await
            .map_err(|e| db_error_to_repo_retrieve_error(e))?;

        Ok(())
    }
}
